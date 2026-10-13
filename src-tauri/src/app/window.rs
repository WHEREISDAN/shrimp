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
