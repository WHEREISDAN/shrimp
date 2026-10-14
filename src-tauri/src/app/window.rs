use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager, Runtime, Window, WindowEvent};

use super::events::MAIN_VISIBLE_EVENT;
use crate::mic::Microphones;
use crate::settings::SettingsStore;

pub const MAIN_LABEL: &str = "main";

/// The main window is open or minimized, not closed to the tray.
static MAIN_OPEN: AtomicBool = AtomicBool::new(false);

pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        log::warn!("main window is missing");
        return;
    };
    MAIN_OPEN.store(true, Ordering::SeqCst);
    update_dock(app);
    let shown = window
        .show()
        .and_then(|()| window.unminimize())
        .and_then(|()| window.set_focus());
    if let Err(err) = shown {
        log::warn!("failed to show main window: {err}");
    }
    let _ = app.emit_to(MAIN_LABEL, MAIN_VISIBLE_EVENT, true);
}

/// Shrimp keeps running in the tray, so closing the main window only hides it.
pub fn on_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event
        && window.label() == MAIN_LABEL
    {
        api.prevent_close();
        if let Err(err) = window.hide() {
            log::warn!("failed to hide main window: {err}");
        }
        MAIN_OPEN.store(false, Ordering::SeqCst);
        update_dock(window.app_handle());
        // A hidden page stays mounted (WebView2 even keeps it "visible"), so the
        // microphone meter of Settings would keep the microphone open.
        if let Some(microphones) = window.app_handle().try_state::<Arc<Microphones>>() {
            microphones.stop_preview();
        }
        let _ = window.emit_to(MAIN_LABEL, MAIN_VISIBLE_EVENT, false);
    }
}

/// "Show app in Dock" (APP-07): a Dock app, or only the menu bar icon. With
/// it off, the Dock icon (and the app's menu bar) still shows while the main
/// window is open or minimized, and goes when it closes to the tray, as
/// Wispr Flow's does (article 1790396454).
pub fn update_dock<R: Runtime>(app: &AppHandle<R>) {
    let in_dock = app
        .try_state::<Arc<SettingsStore>>()
        .is_none_or(|settings| settings.get().app.show_in_dock);
    let show = in_dock || MAIN_OPEN.load(Ordering::SeqCst);
    #[cfg(target_os = "macos")]
    {
        let policy = if show {
            tauri::ActivationPolicy::Regular
        } else {
            tauri::ActivationPolicy::Accessory
        };
        if let Err(err) = app.set_activation_policy(policy) {
            log::warn!("failed to change the Dock icon: {err}");
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = show;
}
