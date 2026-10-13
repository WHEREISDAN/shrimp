use tauri::{AppHandle, Manager, Runtime, Window, WindowEvent};

pub const MAIN_LABEL: &str = "main";

pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        log::warn!("main window is missing");
        return;
    };
    let shown = window
        .show()
        .and_then(|()| window.unminimize())
        .and_then(|()| window.set_focus());
    if let Err(err) = shown {
        log::warn!("failed to show main window: {err}");
    }
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
    }
}

/// "Show app in Dock" (APP-07): a Dock app, or only the menu bar icon.
pub fn show_in_dock<R: Runtime>(app: &AppHandle<R>, show: bool) {
    #[cfg(target_os = "macos")]
    if let Err(err) = app.set_dock_visibility(show) {
        log::warn!("failed to change the Dock icon: {err}");
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, show);
}
