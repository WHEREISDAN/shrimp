//! The macOS menu bar while Shrimp is the active app (APP-10): the app menu,
//! Edit (so text fields in the webview keep their shortcuts), Dictation,
//! Window and Help. Windows has no app menu, so this module is macOS only.
//!
//! Tauri hands every menu's clicks to every menu event handler, so the items
//! this menu shares with the tray keep the tray's ids and the tray's handler
//! runs them; the handler here runs the others.

use tauri::menu::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Runtime};

use super::events::NAVIGATE_EVENT;
use super::{tray, updates, window};
use crate::platform::TRAY_ACCELERATORS;
use crate::t;

const SETTINGS_ID: &str = "menu-settings";
const HELP_ID: &str = "menu-help";
const SETTINGS_KEYS: &str = "CmdOrCtrl+,";
const QUIT_KEYS: &str = "CmdOrCtrl+Q";

/// Sets the menu bar and handles its own items.
pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.on_menu_event(on_menu_event);
    app.set_menu(menu(app)?)?;
    Ok(())
}

/// Builds the menu bar again: the interface language, the update item's
/// status or what there is to copy changed.
pub fn refresh<R: Runtime>(app: &AppHandle<R>) {
    if let Err(error) = menu(app).and_then(|menu| app.set_menu(menu)) {
        log::warn!("could not update the menu bar: {error}");
    }
}

fn on_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let section = match event.id().as_ref() {
        SETTINGS_ID => "settings",
        HELP_ID => "help",
        _ => return,
    };
    window::show_main(app);
    let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, section);
}

fn menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let window_menu = window_menu(app)?;
    let help_menu = help_menu(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &app_menu(app)?,
            &edit_menu(app)?,
            &dictation_menu(app)?,
            &window_menu,
            &help_menu,
        ],
    )?;
    // macOS lists the windows in this menu and adds a search field to Help.
    window_menu.set_as_windows_menu_for_nsapp()?;
    help_menu.set_as_help_menu_for_nsapp()?;
    Ok(menu)
}

fn app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let package = app.package_info();
    let about = AboutMetadata {
        name: Some(package.name.clone()),
        version: Some(package.version.to_string()),
        copyright: app.config().bundle.copyright.clone(),
        ..AboutMetadata::default()
    };
    let item = |id, text, accelerator| MenuItem::with_id(app, id, text, true, Some(accelerator));
    Submenu::with_items(
        app,
        package.name.clone(),
        true,
        &[
            &PredefinedMenuItem::about(app, Some(&t!("menu.about")), Some(about))?,
            &PredefinedMenuItem::separator(app)?,
            &item(SETTINGS_ID, t!("menu.settings"), SETTINGS_KEYS)?,
            &updates::menu_item(app)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, Some(&t!("menu.hide")))?,
            &PredefinedMenuItem::hide_others(app, Some(&t!("menu.hideOthers")))?,
            &PredefinedMenuItem::show_all(app, Some(&t!("menu.showAll")))?,
            &PredefinedMenuItem::separator(app)?,
            &item(tray::QUIT_ID, t!("tray.quit"), QUIT_KEYS)?,
        ],
    )
}

fn edit_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    Submenu::with_items(
        app,
        t!("menu.edit"),
        true,
        &[
            &PredefinedMenuItem::undo(app, Some(&t!("menu.undo")))?,
            &PredefinedMenuItem::redo(app, Some(&t!("menu.redo")))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, Some(&t!("menu.cut")))?,
            &PredefinedMenuItem::copy(app, Some(&t!("menu.copy")))?,
            &PredefinedMenuItem::paste(app, Some(&t!("menu.paste")))?,
            &PredefinedMenuItem::select_all(app, Some(&t!("menu.selectAll")))?,
        ],
    )
}

/// Copy last transcript only: Wispr Flow's Dictation menu has no Paste item.
fn dictation_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let [_, copy_keys, _] = TRAY_ACCELERATORS;
    let copy_last = MenuItem::with_id(
        app,
        tray::COPY_LAST_ID,
        t!("tray.copyLast"),
        tray::copy_last_available(),
        copy_keys,
    )?;
    Submenu::with_items(app, t!("menu.dictation"), true, &[&copy_last])
}

fn window_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    Submenu::with_items(
        app,
        t!("menu.window"),
        true,
        &[
            &PredefinedMenuItem::minimize(app, Some(&t!("menu.minimize")))?,
            &PredefinedMenuItem::maximize(app, Some(&t!("menu.zoom")))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, Some(&t!("menu.close")))?,
        ],
    )
}

fn help_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let item = |id, text| MenuItem::with_id(app, id, text, true, None::<&str>);
    Submenu::with_items(
        app,
        t!("menu.help"),
        true,
        &[
            &item(tray::SHORTCUTS_ID, t!("menu.shortcuts"))?,
            &item(HELP_ID, t!("menu.shrimpHelp"))?,
        ],
    )
}
