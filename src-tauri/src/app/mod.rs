//! App shell: setup, windows, tray, Flow Bar, events and the dev-only MCP bridge.

#[cfg(debug_assertions)]
pub mod dev_bridge;
pub mod events;
pub mod flow_bar;
mod flow_bar_place;
pub mod setup;
mod system;
pub mod tray;
pub mod window;

use std::sync::Arc;

use tauri::{AppHandle, Manager, RunEvent, Runtime};

use crate::health::Health;
use crate::stt::EngineSlot;

pub fn on_run_event<R: Runtime>(app: &AppHandle<R>, event: RunEvent) {
    if let RunEvent::Exit = event {
        // Unload the Whisper model before static destructors run (see EngineSlot::clear).
        if let Some(slot) = app.try_state::<Arc<EngineSlot>>() {
            slot.clear();
        }
        // A quit, not a crash (APP-09).
        if let Some(health) = app.try_state::<Arc<Health>>() {
            health.clean_exit();
        }
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
