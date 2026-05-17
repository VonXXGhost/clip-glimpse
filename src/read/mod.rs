pub mod scanner;
pub mod region;
pub mod ui;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::Path;

use serde::{Serialize, Deserialize};
use anyhow::{Context, anyhow};

use crate::history::History;
use crate::screen::CaptureRegion;
use self::scanner::{Scanner, ScanState};
use self::ui::ReadApp;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScreenRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

impl From<ScreenRegion> for CaptureRegion {
    fn from(s: ScreenRegion) -> Self {
        CaptureRegion::new(s.x, s.y, s.width, s.height)
    }
}

impl From<CaptureRegion> for ScreenRegion {
    fn from(c: CaptureRegion) -> Self {
        Self { x: c.x, y: c.y, width: c.width, height: c.height }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub region: Option<ScreenRegion>,
    pub scan_interval_ms: u64,
    pub hotkey_enabled: bool,
    pub log_enabled: bool,
    pub hotkey: String,
    pub generate_preset_index: usize,
    pub generate_interval_ms: u64,
    pub default_mode: Option<String>,
    pub generate_window_pos: Option<WindowPosition>,
    pub read_window_pos: Option<WindowPosition>,
    pub color_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            region: None,
            scan_interval_ms: 200,
            hotkey_enabled: true,
            log_enabled: true,
            hotkey: "Ctrl+Shift+V".into(),
            generate_preset_index: 1,
            generate_interval_ms: 500,
            default_mode: None,
            generate_window_pos: None,
            read_window_pos: None,
            color_mode: false,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Path::new("config.toml");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        Config::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Path::new("config.toml");

        let mut doc: toml_edit::Document = if path.exists() {
            std::fs::read_to_string(path)
                .context("Failed to read config.toml")?
                .parse()
                .context("Failed to parse config.toml")?
        } else {
            toml_edit::Document::new()
        };

        if let Some(region) = &self.region {
            doc["region"]["x"] = toml_edit::value(region.x as i64);
            doc["region"]["y"] = toml_edit::value(region.y as i64);
            doc["region"]["width"] = toml_edit::value(region.width as i64);
            doc["region"]["height"] = toml_edit::value(region.height as i64);
        } else {
            doc.remove("region");
        }

        // Window positions: only write if Some, never delete if None
        // (each mode only manages its own position)
        if let Some(pos) = &self.generate_window_pos {
            doc["generate_window_pos"]["x"] = toml_edit::value(pos.x as i64);
            doc["generate_window_pos"]["y"] = toml_edit::value(pos.y as i64);
        }
        if let Some(pos) = &self.read_window_pos {
            doc["read_window_pos"]["x"] = toml_edit::value(pos.x as i64);
            doc["read_window_pos"]["y"] = toml_edit::value(pos.y as i64);
        }

        std::fs::write(path, doc.to_string())
            .context("Failed to write config.toml")?;

        Ok(())
    }
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    for path in [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\simsun.ttc",
        r"C:\Windows\Fonts\simhei.ttf",
    ] {
        if let Ok(data) = std::fs::read(path) {
            let name: String = "cjk".into();
            fonts.font_data.insert(name.clone(), std::sync::Arc::new(egui::FontData::from_owned(data)));
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.get_mut(&family).unwrap().insert(0, name.clone());
            }
            break;
        }
    }

    ctx.set_fonts(fonts);
}

pub fn run() -> anyhow::Result<()> {
    let needs_reselect = Arc::new(AtomicBool::new(false));

    loop {
        let mut config = Config::load();
        crate::logger::set_enabled(config.log_enabled);

        let mut scan_region = match config.region {
            Some(r) => r.into(),
            None => {
                let region = region::select_region()
                    .ok_or_else(|| anyhow!("Region selection cancelled"))?;
                config.region = Some(ScreenRegion::from(region));
                config.save()?;
                region
            }
        };
        let history = Arc::new(Mutex::new(History::new()));
        let scan_state = Arc::new(Mutex::new(ScanState::Idle));

        let mut scanner = Scanner::new(
            scan_region,
            history.clone(),
            scan_state.clone(),
            config.scan_interval_ms,
            config.color_mode,
        );
        let stats = scanner.stats();

        scanner.start();

        {
            let hotkey_scan_state = scan_state.clone();
            let hotkey_string = config.hotkey.clone();
            let hotkey_enabled = config.hotkey_enabled;
            let parsed = crate::hotkey::parse_hotkey(&hotkey_string);
            std::thread::Builder::new()
                .name("hotkey-poller".into())
                .spawn(move || {
                    let (hotkey_modifiers, hotkey_vk) = match parsed {
                        Some(p) => p,
                        None => {
                            log_debug!("HOTKEY", "Invalid hotkey string '{}', hotkey disabled", hotkey_string);
                            return;
                        }
                    };
                    let mut was_pressed = false;
                    loop {
                        if !hotkey_enabled {
                            std::thread::sleep(std::time::Duration::from_millis(200));
                            continue;
                        }
                        let pressed = crate::hotkey::is_hotkey_pressed(hotkey_modifiers, hotkey_vk);
                        if pressed && !was_pressed {
                            let mut state = hotkey_scan_state.lock().unwrap();
                            *state = match *state {
                                ScanState::Idle => ScanState::Scanning,
                                ScanState::Scanning => ScanState::Idle,
                            };
                            log_debug!("HOTKEY", "Toggled scanning via hotkey (now {:?})", *state);
                        }
                        was_pressed = pressed;
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                })
                .context("Failed to start hotkey poller thread")?;
        }

        let mut viewport = egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(520.0, 600.0))
            .with_resizable(true)
            .with_icon(crate::icon::create_app_icon());
        if let Some(pos) = config.read_window_pos {
            viewport = viewport.with_position(egui::pos2(pos.x as f32, pos.y as f32));
        }
        let options = eframe::NativeOptions {
            viewport,
            ..Default::default()
        };

        let app = ReadApp::new(
            history,
            scan_state,
            stats,
            scan_region,
            config.clone(),
            needs_reselect.clone(),
        );

        eframe::run_native(
            "ClipGlimpse - Read",
            options,
            Box::new(|cc| {
                setup_fonts(&cc.egui_ctx);
                Ok(Box::new(app))
            }),
        )
        .map_err(|e| anyhow::anyhow!("{}", e))?;

        // If user clicked "Change Region", reopen the region selector
        if needs_reselect.swap(false, Ordering::SeqCst) {
            if let Some(region) = region::select_region() {
                scan_region = region;
                config.region = Some(ScreenRegion::from(region));
                let _ = config.save();
            }
            continue;
        }

        break;
    }

    Ok(())
}
