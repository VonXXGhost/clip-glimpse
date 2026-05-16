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
    pub hotkey_modifiers: u32,
    pub hotkey_vk: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            region: None,
            scan_interval_ms: 200,
            hotkey_enabled: true,
            log_enabled: true,
            hotkey_modifiers: crate::hotkey::HOTKEY_CTRL | crate::hotkey::HOTKEY_SHIFT,
            hotkey_vk: crate::hotkey::VK_V,
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
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        std::fs::write("config.toml", content)
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

    let needs_reselect = Arc::new(AtomicBool::new(false));

    loop {
        let history = Arc::new(Mutex::new(History::new()));
        let scan_state = Arc::new(Mutex::new(ScanState::Idle));
        let hotkey_pressed = Arc::new(AtomicBool::new(false));

        let mut scanner = Scanner::new(
            scan_region,
            history.clone(),
            scan_state.clone(),
            config.scan_interval_ms,
        );
        let stats = scanner.stats();

        scanner.start();

        {
            let hotkey_pressed = hotkey_pressed.clone();
            let hotkey_modifiers = config.hotkey_modifiers;
            let hotkey_vk = config.hotkey_vk;
            let hotkey_enabled = config.hotkey_enabled;
            std::thread::Builder::new()
                .name("hotkey-poller".into())
                .spawn(move || {
                    let mut was_pressed = false;
                    loop {
                        let pressed = hotkey_enabled
                            && crate::hotkey::is_hotkey_pressed(hotkey_modifiers, hotkey_vk);
                        if pressed && !was_pressed {
                            hotkey_pressed.store(true, Ordering::SeqCst);
                        }
                        was_pressed = pressed;
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                })
                .context("Failed to start hotkey poller thread")?;
        }

        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(egui::vec2(520.0, 600.0))
                .with_resizable(true),
            ..Default::default()
        };

        let app = ReadApp::new(
            history,
            scan_state,
            stats,
            hotkey_pressed,
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
