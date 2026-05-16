use eframe::egui;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::history::History;
use crate::screen::CaptureRegion;
use crate::read::scanner::{ScanState, ScanStats};
use crate::read::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Scanner,
    History,
}

pub struct ReadApp {
    history: Arc<Mutex<History>>,
    scan_state: Arc<Mutex<ScanState>>,
    stats: Arc<Mutex<ScanStats>>,
    region: CaptureRegion,
    config: Config,
    selected_tab: Tab,
    selected_index: Option<usize>,
    preview_text: String,
    needs_reselect: Arc<AtomicBool>,
    last_pos_check: std::time::Instant,
    last_saved_pos: Option<(i32, i32)>,
}

impl ReadApp {
    pub fn new(
        history: Arc<Mutex<History>>,
        scan_state: Arc<Mutex<ScanState>>,
        stats: Arc<Mutex<ScanStats>>,
        region: CaptureRegion,
        config: Config,
        needs_reselect: Arc<AtomicBool>,
    ) -> Self {
        Self {
            history,
            scan_state,
            stats,
            region,
            config,
            selected_tab: Tab::Scanner,
            selected_index: None,
            preview_text: String::new(),
            needs_reselect,
            last_pos_check: std::time::Instant::now(),
            last_saved_pos: None,
        }
    }

    fn toggle_scanning(&mut self) {
        let mut state = self.scan_state.lock().unwrap();
        *state = match *state {
            ScanState::Idle => ScanState::Scanning,
            ScanState::Scanning => ScanState::Idle,
        };
    }
}

impl eframe::App for ReadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let self_ = &mut *self;

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self_.selected_tab, Tab::Scanner, "Scanner");
                ui.selectable_value(&mut self_.selected_tab, Tab::History, "History");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self_.selected_tab {
                Tab::Scanner => self_.show_scanner_panel(ui, ctx),
                Tab::History => self_.show_history_panel(ui, ctx),
            }
        });

        if self_.last_pos_check.elapsed() >= std::time::Duration::from_secs(10) {
            self_.last_pos_check = std::time::Instant::now();
            if let Some(rect) = ctx.viewport(|vp| {
                vp.input.raw.viewports.get(&vp.input.raw.viewport_id).and_then(|vi| vi.outer_rect)
            }) {
                let p = (rect.min.x as i32, rect.min.y as i32);
                if self_.last_saved_pos != Some(p) {
                    self_.last_saved_pos = Some(p);
                    self_.config.read_window_pos = Some(crate::read::WindowPosition {
                        x: p.0, y: p.1,
                    });
                    let _ = self_.config.save();
                }
            }
        }
    }
}

impl ReadApp {
    fn show_scanner_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Scanner");
        ui.add_space(8.0);

        let is_scanning = *self.scan_state.lock().unwrap() == ScanState::Scanning;

        ui.horizontal(|ui| {
            let btn_text = if is_scanning { "\u{25A0} Stop Scan" } else { "\u{25B6} Start Scan" };
            if ui.button(btn_text).clicked() {
                log_debug!("READ", "{} clicked", if is_scanning { "Stop Scan" } else { "Start Scan" });
                self.toggle_scanning();
            }

            let color = if is_scanning {
                egui::Color32::GREEN
            } else {
                egui::Color32::GRAY
            };
            ui.label(egui::RichText::new(if is_scanning { " SCANNING" } else { " IDLE" })
                .color(color)
                .strong());
        });

        ui.add_space(12.0);

        ui.label(format!("Scan region: ({}, {}) {}x{}",
            self.region.x, self.region.y, self.region.width, self.region.height));

        ui.add_space(4.0);

        if ui.button("Change Region").clicked() {
            self.needs_reselect.store(true, Ordering::SeqCst);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        ui.add_space(16.0);

        ui.label(format!("Hotkey: {}", crate::hotkey::normalize_hotkey(&self.config.hotkey)));
        if self.config.hotkey_enabled {
            ui.label("Hotkey polling is enabled");
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        ui.heading("Statistics");
        let stats = self.stats.lock().unwrap().clone();
        ui.label(format!("Frames captured: {}", stats.frames_captured));
        ui.label(format!("Frames decoded:  {}", stats.frames_decoded));
        if stats.current_message_active && stats.current_total_chunks > 0 {
            let progress = stats.current_received_data_chunks.min(stats.current_total_chunks);
            ui.label(format!("Chunk progress: {}/{}", progress, stats.current_total_chunks));
        } else {
            ui.label(format!("Chunks received: {}", stats.chunks_received));
        }
        ui.label(format!("Messages completed: {}", stats.messages_completed));
        if let Some(ref t) = stats.last_message_time {
            ui.label(format!("Last message: {}", t));
        }

        if is_scanning {
            ctx.request_repaint();
        }
    }

    fn show_history_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("History");
        ui.add_space(8.0);

        let entries = {
            let h = self.history.lock().unwrap();
            h.entries().to_vec()
        };

        if entries.is_empty() {
            ui.label("No messages received yet.");
            ui.label("Go to the Scanner tab and start scanning QR codes.");
        } else {
            egui::ScrollArea::vertical()
                .max_height(ui.available_height() - 180.0)
                .show(ui, |ui| {
                    for (i, entry) in entries.iter().enumerate() {
                        let selected = self.selected_index == Some(i);
                        let text = format!("[{}] {}", entry.timestamp, entry.preview(80));
                        let response = ui.selectable_label(selected, &text);
                        if response.clicked() {
                            self.selected_index = Some(i);
                            self.preview_text = entry.text.clone();
                        }
                    }
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            if !self.preview_text.is_empty() {
                ui.label("Selected message:");
                egui::Frame::default()
                    .fill(egui::Color32::from_white_alpha(240))
                    .stroke(egui::epaint::Stroke::new(1.0, egui::Color32::LIGHT_GRAY))
                    .inner_margin(4.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(100.0)
                            .show(ui, |ui| {
                                ui.label(&self.preview_text);
                            });
                    });
            }

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let has_selection = self.selected_index.is_some();
                let copy_btn = ui.add_enabled(has_selection, egui::Button::new("Copy to Clipboard"));
                if copy_btn.clicked() {
                    if let Some(idx) = self.selected_index {
                        if let Some(entry) = entries.get(idx) {
                            ctx.copy_text(entry.text.clone());
                        }
                    }
                }

                if ui.button("Clear All").clicked() {
                    let mut h = self.history.lock().unwrap();
                    h.clear();
                    self.selected_index = None;
                    self.preview_text.clear();
                }

                if ui.button("Refresh").clicked() {
                    ctx.request_repaint();
                }
            });
        }
    }
}
