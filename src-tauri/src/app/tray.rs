use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime, include_image};

use super::events::{NAVIGATE_EVENT, TauriUi};
use super::{updates, window};
use crate::commands::system;
use crate::dictation::{ControllerHandle, ControllerInput, DictationUi};
use crate::hotkeys::Action;
use crate::notify::Notice;
use crate::platform::{TRAY_ACCELERATORS, TRAY_CLICK_OPENS_WINDOW};
use crate::settings::{Settings, SettingsStore};
use crate::t;

const TRAY_ID: &str = "main";
const TRAY_ICON: Image<'_> = include_image!("icons/tray.png");
const OPEN_ID: &str = "open";
const SHORTCUTS_ID: &str = "shortcuts";
const PASTE_LAST_ID: &str = "paste-last";
const COPY_LAST_ID: &str = "copy-last";
const PRIVACY_ID: &str = "privacy-mode";
const RESTART_ID: &str = "restart";
const QUIT_ID: &str = "quit";

/// Paste last transcript is greyed out until there is one (WIN-09).
static PASTE_LAST_AVAILABLE: AtomicBool = AtomicBool::new(false);

pub fn create<R: Runtime>(app: &AppHandle<R>, settings: &Settings) -> tauri::Result<()> {
    let privacy = settings.privacy.privacy_mode;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon(privacy))
        .icon_as_template(true)
        .tooltip(tooltip(privacy))
        .menu(&menu(app, settings)?)
        .show_menu_on_left_click(!TRAY_CLICK_OPENS_WINDOW)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_ID => window::show_main(app),
            SHORTCUTS_ID => {
                window::show_main(app);
                let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "shortcuts");
            }
            PASTE_LAST_ID => send(app, Action::PasteLast),
            COPY_LAST_ID => send(app, Action::CopyLast),
            PRIVACY_ID => toggle_privacy_mode(app),
            updates::MENU_ID => updates::menu_clicked(app),
            RESTART_ID => restart(app),
            QUIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
                && TRAY_CLICK_OPENS_WINDOW
            {
                window::show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Shows Privacy mode in the menu, the icon and the tooltip (PRIV-03).
pub fn refresh<R: Runtime>(app: &AppHandle<R>, settings: &Settings) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let privacy = settings.privacy.privacy_mode;
    let updated = menu(app, settings)
        .and_then(|menu| tray.set_menu(Some(menu)))
        .and_then(|()| tray.set_icon_with_as_template(Some(icon(privacy)), true))
        .and_then(|()| tray.set_tooltip(Some(tooltip(privacy))));
    if let Err(error) = updated {
        log::warn!("could not update the tray: {error}");
    }
}

/// Enables or greys out Paste last transcript.
pub fn set_paste_last_available<R: Runtime>(app: &AppHandle<R>, available: bool) {
    if PASTE_LAST_AVAILABLE.swap(available, Ordering::Relaxed) == available {
        return;
    }
    if let Some(settings) = app.try_state::<Arc<SettingsStore>>() {
        let settings = settings.get();
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || refresh(&handle, &settings));
    }
}

fn menu<R: Runtime>(app: &AppHandle<R>, settings: &Settings) -> tauri::Result<Menu<R>> {
    let item = |id, text, enabled, accelerator: Option<&str>| {
        MenuItem::with_id(app, id, text, enabled, accelerator)
    };
    let [paste_keys, copy_keys, quit_keys] = TRAY_ACCELERATORS;
    let paste_available = PASTE_LAST_AVAILABLE.load(Ordering::Relaxed);
    log::debug!("tray menu: paste last transcript enabled: {paste_available}");
    let open = item(OPEN_ID, t!("tray.open"), true, None)?;
    let shortcuts = item(SHORTCUTS_ID, t!("tray.shortcuts"), true, None)?;
    let update = updates::menu_item(app)?;
    let paste_last = item(
        PASTE_LAST_ID,
        t!("tray.pasteLast"),
        paste_available,
        paste_keys,
    )?;
    let copy_last = item(COPY_LAST_ID, t!("tray.copyLast"), true, copy_keys)?;
    let privacy = CheckMenuItem::with_id(
        app,
        PRIVACY_ID,
        t!("tray.privacyMode"),
        true,
        settings.privacy.privacy_mode,
        None::<&str>,
    )?;
    let restart = item(RESTART_ID, t!("tray.restart"), true, None)?;
    let quit = item(QUIT_ID, t!("tray.quit"), true, quit_keys)?;
    Menu::with_items(
        app,
        &[
            &open,
            &shortcuts,
            &update,
            &PredefinedMenuItem::separator(app)?,
            &paste_last,
            &copy_last,
            &PredefinedMenuItem::separator(app)?,
            &privacy,
            &PredefinedMenuItem::separator(app)?,
            &restart,
            &quit,
        ],
    )
}

fn tooltip(privacy: bool) -> String {
    if privacy {
        t!("tray.privacyTooltip")
    } else {
        "Shrimp".into()
    }
}

/// The template icon, with a dot in the corner while Privacy mode is on.
fn icon(privacy: bool) -> Image<'static> {
    let (width, height) = (TRAY_ICON.width(), TRAY_ICON.height());
    let mut rgba = TRAY_ICON.rgba().to_vec();
    if privacy {
        let size = width.min(height) as f32;
        let (cx, cy) = (size * 0.8, size * 0.8);
        for y in 0..height {
            for x in 0..width {
                let distance = (x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy);
                let alpha = if distance <= size * 0.16 {
                    255
                } else if distance <= size * 0.22 {
                    0
                } else {
                    continue;
                };
                let at = ((y * width + x) * 4) as usize;
                rgba[at..at + 4].copy_from_slice(&[0, 0, 0, alpha]);
            }
        }
    }
    Image::new_owned(rgba, width, height)
}

fn toggle_privacy_mode<R: Runtime>(app: &AppHandle<R>) {
    let Some(settings) = app.try_state::<Arc<SettingsStore>>() else {
        return;
    };
    let on = !settings.get().privacy.privacy_mode;
    if let Err(error) = settings.update(&serde_json::json!({"privacy": {"privacyMode": on}})) {
        log::warn!("could not switch Privacy mode: {error}");
    }
}

/// "Restart Shrimp" (APP-12): a normal quit, so Whisper unloads and no crash
/// screen follows, then the app opens again with its main window, without
/// asking first (Wispr Flow's Restart Flow, article 7492559320).
fn restart<R: Runtime>(app: &AppHandle<R>) {
    let Err(error) = system::restart(app) else {
        return;
    };
    log::error!("could not restart: {}", error.message);
    if let Some(ui) = app.try_state::<Arc<TauriUi>>() {
        ui.notice(Notice::error(t!(
            "notice.restartFailed",
            error = error.message
        )));
    }
}

fn send<R: Runtime>(app: &AppHandle<R>, action: Action) {
    match app.try_state::<ControllerHandle>() {
        Some(controller) => controller.send(ControllerInput::Menu(action)),
        None => log::warn!("dictation is not running"),
    }
}
