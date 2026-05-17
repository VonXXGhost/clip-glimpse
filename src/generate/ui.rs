use eframe::egui;
use qrcode::{EcLevel, Version};
use crate::protocol::{self, encode_message, MAX_CHUNKS};
use crate::qr_gen::{self, QrGenParams, generate_color_qr};

const MAX_CYCLES: u64 = 100;

struct Preset {
    name: &'static str,
    version: Version,
    ec_level: EcLevel,
    module_size_px: u32,
    qr_capacity: usize,
}

impl Preset {
    const fn payload_size(&self) -> usize {
        self.qr_capacity.saturating_sub(protocol::HEADER_SIZE)
    }
}

static PRESETS: &[Preset] = &[
    Preset { name: "Conservative V20-Q", version: Version::Normal(20), ec_level: EcLevel::Q, module_size_px: 3, qr_capacity: 427 },
    Preset { name: "Default V25-M",      version: Version::Normal(25), ec_level: EcLevel::M, module_size_px: 3, qr_capacity: 779 },
    Preset { name: "Fast V30-M",         version: Version::Normal(30), ec_level: EcLevel::M, module_size_px: 2, qr_capacity: 1043 },
    Preset { name: "Extreme V35-L",      version: Version::Normal(35), ec_level: EcLevel::L, module_size_px: 2, qr_capacity: 1595 },
];

const INTERVALS_MS: &[u64] = &[200, 300, 500, 800, 1000];

struct DisplayChunk {
    qr_image: Option<image::RgbImage>,
}

impl DisplayChunk {
    fn new(data: Vec<u8>, params: &QrGenParams) -> Self {
        let qr_image = qr_gen::generate_qr(&data, params);
        Self { qr_image }
    }

    fn new_color(group: Vec<Vec<u8>>, params: &QrGenParams) -> Self {
        let qr_image = generate_color_qr(&group, params);
        Self { qr_image }
    }

    fn texture(&self, ctx: &egui::Context, label: &str) -> Option<egui::TextureHandle> {
        let img = self.qr_image.as_ref()?;
        let color_image = qr_gen::qr_to_egui_color_image(img);
        Some(ctx.load_texture(label, color_image, egui::TextureOptions::NEAREST))
    }
}

pub struct GenerateApp {
    input_text: String,
    preset_index: usize,
    interval_index: usize,
    is_running: bool,
    frames: Vec<DisplayChunk>,
    current_index: usize,
    last_cycle: std::time::Instant,
    qr_texture: Option<egui::TextureHandle>,
    status_message: String,
    cycle_count: u64,
    config: crate::read::Config,
    last_pos_check: std::time::Instant,
    last_saved_pos: Option<(i32, i32)>,
    color_mode: bool,
    raw_chunk_count: usize,
}

impl GenerateApp {
    pub fn with_config(config: &crate::read::Config) -> Self {
        let interval_index = INTERVALS_MS.iter()
            .position(|&ms| ms == config.generate_interval_ms)
            .unwrap_or_else(|| {
                INTERVALS_MS.iter()
                    .enumerate()
                    .min_by_key(|&(_, &ms)| (ms as i64 - config.generate_interval_ms as i64).abs())
                    .map(|(i, _)| i)
                    .unwrap_or(2)
            });
        Self {
            input_text: String::new(),
            preset_index: config.generate_preset_index.min(PRESETS.len().saturating_sub(1)),
            interval_index,
            is_running: false,
            frames: Vec::new(),
            current_index: 0,
            last_cycle: std::time::Instant::now(),
            qr_texture: None,
            status_message: String::new(),
            cycle_count: 0,
            config: config.clone(),
            last_pos_check: std::time::Instant::now(),
            last_saved_pos: None,
            color_mode: config.color_mode,
            raw_chunk_count: 0,
        }
    }
}

impl GenerateApp {
    fn start_cycling(&mut self, ctx: &egui::Context) {
        if self.frames.len() <= 1 {
            return;
        }
        self.is_running = true;
        self.current_index = 0;
        self.cycle_count = 0;
        self.last_cycle = std::time::Instant::now();
        self.status_message.clear();
        self.show_qr_preview(ctx);
        log_debug!("GEN", "Start cycling: {} frames, {}ms interval",
            self.frames.len(), self.interval_ms());
        ctx.request_repaint();
    }

    fn sync_display_state(&mut self, ctx: &egui::Context) {
        if self.frames.is_empty() {
            self.is_running = false;
            self.qr_texture = None;
        } else if self.frames.len() == 1 {
            self.is_running = false;
            self.current_index = 0;
            self.cycle_count = 0;
            self.show_qr_preview(ctx);
        } else {
            self.start_cycling(ctx);
        }
    }
}

impl GenerateApp {
    fn preset(&self) -> &Preset {
        &PRESETS[self.preset_index]
    }

    fn interval_ms(&self) -> u64 {
        INTERVALS_MS[self.interval_index]
    }

    fn qr_params(&self) -> QrGenParams {
        let preset = self.preset();
        QrGenParams {
            version: preset.version,
            ec_level: preset.ec_level,
            module_size_px: preset.module_size_px,
        }
    }

    fn rebuild_frames(&mut self) {
        self.frames.clear();
        if self.input_text.is_empty() {
            log_debug!("GEN", "Rebuild: empty text, no frames");
            self.raw_chunk_count = 0;
            return;
        }

        let payload_size = self.preset().payload_size();
        let params = self.qr_params();
        let raw_chunks = encode_message(&self.input_text, payload_size);

        if raw_chunks.len() > MAX_CHUNKS as usize {
            self.status_message = format!("Warning: {} chunks needed, max is {}. Text too long.", raw_chunks.len(), MAX_CHUNKS);
            log_debug!("GEN", "Rebuild: TOO LONG, {} chunks needed", raw_chunks.len());
            self.raw_chunk_count = 0;
            return;
        }

        self.raw_chunk_count = raw_chunks.len();
        let total_size = self.input_text.len();
        let encoded: Vec<Vec<u8>> = raw_chunks.iter().map(|c| c.encode()).collect();

        if self.color_mode {
            let frames_count = (encoded.len() + 2) / 3;
            let estimated_time = frames_count as f64 * self.interval_ms() as f64 / 1000.0;
            self.status_message = format!(
                "{} chunks ({} frames), {} bytes, ~{:.1}s per cycle",
                encoded.len(), frames_count, total_size, estimated_time
            );

            for group in encoded.chunks(3) {
                self.frames.push(DisplayChunk::new_color(group.to_vec(), &params));
            }

            log_debug!("GEN", "Rebuild color: {} chunks, {} frames, {} bytes, interval={}ms",
                encoded.len(), frames_count, total_size, self.interval_ms());
        } else {
            let estimated_time = encoded.len() as f64 * self.interval_ms() as f64 / 1000.0;
            self.status_message = format!(
                "{} chunks, {} bytes, ~{:.1}s per cycle",
                encoded.len(), total_size, estimated_time
            );

            for data in &encoded {
                self.frames.push(DisplayChunk::new(data.clone(), &params));
            }

            log_debug!("GEN", "Rebuild: {} chunks, {} bytes, interval={}ms",
                encoded.len(), total_size, self.interval_ms());
        }
    }

    fn show_qr_preview(&mut self, ctx: &egui::Context) {
        if !self.frames.is_empty() && self.current_index < self.frames.len() {
            let frame = &self.frames[self.current_index];
            self.qr_texture = frame.texture(ctx, &format!("qr_{}", self.current_index));
        }
    }
}

impl eframe::App for GenerateApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let self_ = &mut *self;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ClipGlimpse - Generate");
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .id_salt("text_scroll")
                .max_height(150.0)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self_.input_text)
                            .desired_rows(8)
                            .desired_width(f32::INFINITY)
                            .hint_text("Paste or type your text here...")
                            .frame(true)
                    );
                    if response.changed() {
                        log_debug!("GEN", "Text changed, len={}", self_.input_text.len());
                        self_.rebuild_frames();
                        self_.sync_display_state(ctx);
                    }
                });

            ui.add_space(12.0);

            let qr_frame = egui::Frame::default()
                .fill(egui::Color32::from_white_alpha(240))
                .stroke(egui::epaint::Stroke::new(1.0, egui::Color32::GRAY))
                .inner_margin(8.0);

            qr_frame.show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    let display_size = egui::vec2(ui.available_width().min(400.0), 320.0);
                    let (rect, _) = ui.allocate_exact_size(display_size, egui::Sense::hover());

                    if !self_.frames.is_empty() {
                        if let Some(tex) = &self_.qr_texture {
                            let size = tex.size_vec2();
                            let max_w = rect.width() - 16.0;
                            let max_h = rect.height() - 16.0;
                            let scale = (max_w / size.x).min(max_h / size.y).min(1.0);
                            let img_size = size * scale;
                            let img_rect = egui::Rect::from_center_size(rect.center(), img_size);
                            ui.put(img_rect, egui::Image::new(tex).fit_to_exact_size(img_size));
                        } else {
                            self_.show_qr_preview(ctx);
                        }

                        if self_.is_running {
                            let label = if self_.color_mode {
                                format!("Frame {}/{} ({} chunks, cycle #{})",
                                    self_.current_index + 1,
                                    self_.frames.len(),
                                    self_.raw_chunk_count,
                                    self_.cycle_count
                                )
                            } else {
                                format!("Chunk {}/{} (cycle #{})",
                                    self_.current_index + 1,
                                    self_.frames.len(),
                                    self_.cycle_count
                                )
                            };
                            ui.label(label);
                        }
                    } else {
                        let painter = ui.painter();
                        painter.text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Enter text to generate QR code",
                            egui::FontId::proportional(16.0),
                            egui::Color32::GRAY,
                        );
                    }
                });
            });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.label("Preset:");
                egui::ComboBox::from_id_salt("preset")
                    .selected_text(self_.preset().name)
                    .show_ui(ui, |ui| {
                        for (i, preset) in PRESETS.iter().enumerate() {
                            let selected = ui.selectable_label(self_.preset_index == i, preset.name);
                            if selected.clicked() {
                                log_debug!("GEN", "Preset changed to: {}", preset.name);
                                self_.preset_index = i;
                                self_.rebuild_frames();
                                self_.sync_display_state(ctx);
                            }
                        }
                    });

                ui.label("Interval:");
                egui::ComboBox::from_id_salt("interval")
                    .selected_text(format!("{}ms", self_.interval_ms()))
                    .show_ui(ui, |ui| {
                        for (i, &ms) in INTERVALS_MS.iter().enumerate() {
                            let selected = ui.selectable_label(self_.interval_index == i, format!("{}ms", ms));
                            if selected.clicked() {
                                self_.interval_index = i;
                            }
                        }
                    });

                if ui.checkbox(&mut self_.color_mode, "Color (3x)").changed() {
                    self_.config.color_mode = self_.color_mode;
                    let _ = self_.config.save();
                    if !self_.input_text.is_empty() {
                        self_.rebuild_frames();
                        self_.sync_display_state(ctx);
                    }
                }
            });

            ui.add_space(8.0);

            if self_.frames.len() > 1 {
                let btn_text = if self_.is_running { "\u{23F8} Pause" } else { "\u{25B6} Resume" };
                if ui.button(btn_text).clicked() {
                    if self_.is_running {
                        log_debug!("GEN", "Pause cycling");
                        self_.is_running = false;
                    } else {
                        log_debug!("GEN", "Resume cycling");
                        self_.start_cycling(ctx);
                    }
                }
            }

            ui.add_space(4.0);

            if !self_.status_message.is_empty() {
                ui.label(
                    egui::RichText::new(&self_.status_message)
                        .color(egui::Color32::DARK_BLUE)
                        .size(12.0)
                );
            }
        });

        if self_.is_running && !self_.frames.is_empty() {
            let elapsed = self_.last_cycle.elapsed();
            if elapsed.as_millis() >= self_.interval_ms() as u128 {
                self_.current_index = (self_.current_index + 1) % self_.frames.len();
                if self_.current_index == 0 {
                    self_.cycle_count += 1;
                    if self_.cycle_count >= MAX_CYCLES {
                        log_debug!("GEN", "Auto-pause after {} cycles", MAX_CYCLES);
                        self_.is_running = false;
                        self_.status_message = format!("Paused after {} cycles (max limit reached)", MAX_CYCLES);
                    }
                }
                self_.last_cycle = std::time::Instant::now();
                log_debug!("GEN", "Show frame {}/{} (cycle #{})",
                    self_.current_index + 1, self_.frames.len(), self_.cycle_count);
                self_.show_qr_preview(ctx);
            }
            ctx.request_repaint();
        }

        if self_.last_pos_check.elapsed() >= std::time::Duration::from_secs(10) {
            self_.last_pos_check = std::time::Instant::now();
            if let Some(rect) = ctx.viewport(|vp| {
                vp.input.raw.viewports.get(&vp.input.raw.viewport_id).and_then(|vi| vi.outer_rect)
            }) {
                let p = (rect.min.x as i32, rect.min.y as i32);
                if self_.last_saved_pos != Some(p) {
                    self_.last_saved_pos = Some(p);
                    self_.config.generate_window_pos = Some(crate::read::WindowPosition {
                        x: p.0, y: p.1,
                    });
                    let _ = self_.config.save();
                }
            }
        }
    }
}
