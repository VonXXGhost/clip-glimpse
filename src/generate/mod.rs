pub mod ui;


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
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(480.0, 720.0))
            .with_resizable(false),
        ..Default::default()
    };

    let config = crate::read::Config::load();

    eframe::run_native(
        "ClipGlimpse - Generate",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(ui::GenerateApp::with_config(&config)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(())
}
