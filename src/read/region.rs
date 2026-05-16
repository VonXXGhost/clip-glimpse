use eframe::egui;
use std::sync::mpsc;

use crate::screen::{self, CaptureRegion};

pub struct RegionSelector {
    screenshot: egui::ColorImage,
    drag_start: Option<egui::Pos2>,
    drag_end: Option<egui::Pos2>,
    is_dragging: bool,
    result_tx: Option<mpsc::Sender<Option<CaptureRegion>>>,
    scale: f32,
}

impl RegionSelector {
    pub fn new(
        screenshot: egui::ColorImage,
        tx: mpsc::Sender<Option<CaptureRegion>>,
    ) -> Self {
        Self {
            screenshot,
            drag_start: None,
            drag_end: None,
            is_dragging: false,
            result_tx: Some(tx),
            scale: 1.0,
        }
    }

    fn selected_rect(&self) -> Option<egui::Rect> {
        match (self.drag_start, self.drag_end) {
            (Some(a), Some(b)) => Some(egui::Rect::from_two_pos(a, b)),
            _ => None,
        }
    }

    fn region_from_rect(&self, rect: egui::Rect) -> CaptureRegion {
        let x = (rect.min.x / self.scale) as i32;
        let y = (rect.min.y / self.scale) as i32;
        let w = (rect.width() / self.scale) as u32;
        let h = (rect.height() / self.scale) as u32;
        CaptureRegion::new(x, y, w, h)
    }
}

impl eframe::App for RegionSelector {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let self_ = &mut *self;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Select QR Scan Region");
            ui.label("Drag a rectangle around the QR code area on your screen.");
            ui.add_space(8.0);

            let available = ui.available_size();
            let img_aspect = self_.screenshot.size[0] as f32 / self_.screenshot.size[1] as f32;
            let max_w = available.x;
            let max_h = available.y - 60.0;
            let mut display_w = max_w;
            let mut display_h = display_w / img_aspect;
            if display_h > max_h {
                display_h = max_h;
                display_w = display_h * img_aspect;
            }
            self_.scale = display_w / self_.screenshot.size[0] as f32;

            let texture = ctx.load_texture(
                "screenshot",
                self_.screenshot.clone(),
                egui::TextureOptions::LINEAR,
            );

            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(display_w, display_h),
                egui::Sense::click_and_drag(),
            );

            ui.put(rect, egui::Image::new(&texture).fit_to_exact_size(egui::vec2(display_w, display_h)));

            let painter = ui.painter();
            let overlay_rect = rect;

            if let Some(sel_rect) = self_.selected_rect() {
                let clipped = sel_rect.intersect(overlay_rect);

                let mut outside_rects = Vec::new();
                if clipped.min.y > overlay_rect.min.y {
                    outside_rects.push(egui::Rect::from_min_max(
                        egui::pos2(overlay_rect.min.x, overlay_rect.min.y),
                        egui::pos2(overlay_rect.max.x, clipped.min.y),
                    ));
                }
                if clipped.max.y < overlay_rect.max.y {
                    outside_rects.push(egui::Rect::from_min_max(
                        egui::pos2(overlay_rect.min.x, clipped.max.y),
                        egui::pos2(overlay_rect.max.x, overlay_rect.max.y),
                    ));
                }
                if clipped.min.x > overlay_rect.min.x {
                    outside_rects.push(egui::Rect::from_min_max(
                        egui::pos2(overlay_rect.min.x, clipped.min.y),
                        egui::pos2(clipped.min.x, clipped.max.y),
                    ));
                }
                if clipped.max.x < overlay_rect.max.x {
                    outside_rects.push(egui::Rect::from_min_max(
                        egui::pos2(clipped.max.x, clipped.min.y),
                        egui::pos2(overlay_rect.max.x, clipped.max.y),
                    ));
                }

                for r in &outside_rects {
                    painter.rect_filled(*r, 0.0, egui::Color32::from_black_alpha(120));
                }

                painter.rect_stroke(
                    clipped,
                    0.0,
                    egui::epaint::Stroke::new(2.0, egui::Color32::RED),
                    egui::StrokeKind::Inside,
                );
            }

            if response.dragged() {
                let pos = response.interact_pointer_pos().unwrap_or(egui::Pos2::ZERO);
                let clamped = egui::pos2(
                    pos.x.clamp(overlay_rect.min.x, overlay_rect.max.x),
                    pos.y.clamp(overlay_rect.min.y, overlay_rect.max.y),
                );

                if response.drag_started() {
                    self_.drag_start = Some(clamped);
                    self_.drag_end = Some(clamped);
                    self_.is_dragging = true;
                } else if self_.is_dragging {
                    self_.drag_end = Some(clamped);
                }
            }

            if response.drag_stopped() {
                self_.is_dragging = false;
            }

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    if let Some(tx) = self_.result_tx.take() {
                        let _ = tx.send(None);
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                let region_ready = self_.selected_rect().is_some();
                if ui.add_enabled(region_ready, egui::Button::new("OK")).clicked() {
                    if let Some(rect) = self_.selected_rect() {
                        let region = self_.region_from_rect(rect);
                        if let Some(tx) = self_.result_tx.take() {
                            let _ = tx.send(Some(region));
                        }
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            if let Some(rect) = self_.selected_rect() {
                let r = self_.region_from_rect(rect);
                ui.label(format!(
                    "Selected region: ({}, {}) {}x{}",
                    r.x, r.y, r.width, r.height
                ));
            }
        });
    }
}

pub fn select_region() -> Option<CaptureRegion> {
    let (tx, rx) = mpsc::channel();

    let (screen_pixels, w, h) = match capture_full_screen() {
        Some(data) => data,
        None => return None,
    };

    let color_image = create_color_image_from_bgra(&screen_pixels, w, h);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_fullscreen(true)
            .with_title("Select QR Scan Region"),
        ..Default::default()
    };

    let result = eframe::run_native(
        "Select QR Scan Region",
        options,
        Box::new(|_cc| {
            Ok(Box::new(RegionSelector::new(color_image, tx)))
        }),
    );

    if let Err(e) = result {
        eprintln!("Region selection window error: {}", e);
    }

    match rx.recv() {
        Ok(Some(region)) => Some(region),
        _ => None,
    }
}

fn capture_full_screen() -> Option<(Vec<u8>, u32, u32)> {
    let (w, h) = screen::get_screen_size().ok()?;
    let region = CaptureRegion::new(0, 0, w, h);
    let pixels = screen::capture_region(&region).ok()?;
    Some((pixels, w, h))
}

fn create_color_image_from_bgra(data: &[u8], width: u32, height: u32) -> egui::ColorImage {
    let size = [width as usize, height as usize];
    let pixels: Vec<egui::Color32> = data
        .chunks(4)
        .map(|p| egui::Color32::from_rgba_unmultiplied(p[2], p[1], p[0], 255))
        .collect();
    egui::ColorImage { size, pixels }
}
