use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Runtime, include_image};

use super::window;

const TRAY_ID: &str = "main";
const TRAY_ICON: Image<'_> = include_image!("icons/tray.png");
const OPEN_ID: &str = "open";
const QUIT_ID: &str = "quit";

pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, OPEN_ID, "Open Shrimp", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Quit Shrimp", true, Some("CmdOrCtrl+Q"))?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &separator, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(TRAY_ICON)
        .icon_as_template(true)
        .tooltip("Shrimp")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_ID => window::show_main(app),
            QUIT_ID => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
