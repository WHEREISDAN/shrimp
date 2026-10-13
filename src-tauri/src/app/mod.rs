//! App shell: setup, windows, tray, Flow Bar, events and the dev-only MCP bridge.

#[cfg(debug_assertions)]
pub mod dev_bridge;
pub mod events;
pub mod flow_bar;
pub mod setup;
pub mod tray;
pub mod window;

use std::sync::Arc;

use tauri::{AppHandle, Manager, RunEvent, Runtime};

use crate::stt::EngineSlot;

pub fn on_run_event<R: Runtime>(app: &AppHandle<R>, event: RunEvent) {
    // Unload the Whisper model before static destructors run (see EngineSlot::clear).
    if let RunEvent::Exit = event
        && let Some(slot) = app.try_state::<Arc<EngineSlot>>()
    {
        slot.clear();
    }
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
