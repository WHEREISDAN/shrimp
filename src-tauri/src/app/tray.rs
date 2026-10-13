use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Runtime, include_image};

use super::events::NAVIGATE_EVENT;
use super::window;
use crate::dictation::{ControllerHandle, ControllerInput};
use crate::hotkeys::Action;

const TRAY_ID: &str = "main";
const TRAY_ICON: Image<'_> = include_image!("icons/tray.png");
const OPEN_ID: &str = "open";
const SHORTCUTS_ID: &str = "shortcuts";
const PASTE_LAST_ID: &str = "paste-last";
const COPY_LAST_ID: &str = "copy-last";
const QUIT_ID: &str = "quit";

pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let item =
        |id, text, accelerator: Option<&str>| MenuItem::with_id(app, id, text, true, accelerator);
    let open = item(OPEN_ID, "Open Shrimp", None)?;
    let shortcuts = item(SHORTCUTS_ID, "Shortcuts…", None)?;
    let paste_last = item(PASTE_LAST_ID, "Paste last transcript", Some("Ctrl+Cmd+V"))?;
    let copy_last = item(COPY_LAST_ID, "Copy last transcript", Some("Ctrl+Cmd+C"))?;
    let quit = item(QUIT_ID, "Quit Shrimp", Some("CmdOrCtrl+Q"))?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &shortcuts,
            &PredefinedMenuItem::separator(app)?,
            &paste_last,
            &copy_last,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(TRAY_ICON)
        .icon_as_template(true)
        .tooltip("Shrimp")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_ID => window::show_main(app),
            SHORTCUTS_ID => {
                window::show_main(app);
                let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "shortcuts");
            }
            PASTE_LAST_ID => send(app, Action::PasteLast),
            COPY_LAST_ID => send(app, Action::CopyLast),
            QUIT_ID => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn send<R: Runtime>(app: &AppHandle<R>, action: Action) {
    match app.try_state::<ControllerHandle>() {
        Some(controller) => controller.send(ControllerInput::Menu(action)),
        None => log::warn!("dictation is not running"),
    }
}
