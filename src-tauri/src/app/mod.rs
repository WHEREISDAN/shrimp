//! App shell: setup, windows, tray, Flow Bar, events and the dev-only MCP bridge.

#[cfg(debug_assertions)]
pub mod dev_bridge;
pub mod events;
pub mod flow_bar;
pub mod setup;
pub mod tray;
pub mod window;

use tauri::{AppHandle, RunEvent, Runtime};

#[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
pub fn on_run_event<R: Runtime>(app: &AppHandle<R>, event: RunEvent) {
    // macOS: clicking the Dock icon while every window is hidden.
    #[cfg(target_os = "macos")]
    if let RunEvent::Reopen {
        has_visible_windows: false,
        ..
    } = event
    {
        window::show_main(app);
    }
}
