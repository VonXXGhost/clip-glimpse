use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuItem},
};

pub struct AppTray {
    pub tray: TrayIcon,
}

impl AppTray {
    pub fn new() -> anyhow::Result<Self> {
        let menu = Menu::new();

        let open_item = MenuItem::new("Open History", true, None);
        let quit_item = MenuItem::new("Quit", true, None);
        menu.append_items(&[&open_item, &quit_item])?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("ClipGlimpse")
            .build()?;

        Ok(Self { tray })
    }
}
