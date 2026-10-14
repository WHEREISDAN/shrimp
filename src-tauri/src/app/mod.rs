//! App shell: setup, windows, tray, the macOS menu bar, Flow Bar, events,
//! updates and the dev-only MCP bridge.

#[cfg(debug_assertions)]
pub mod dev_bridge;
pub mod events;
pub mod flow_bar;
mod flow_bar_place;
#[cfg(target_os = "macos")]
mod menu_bar;
pub mod setup;
mod system;
pub mod tray;
pub mod updates;
pub mod window;

use std::sync::Arc;

use tauri::{AppHandle, Manager, RunEvent, Runtime};

use crate::health::Health;
use crate::stt::EngineSlot;

pub fn on_run_event<R: Runtime>(app: &AppHandle<R>, event: RunEvent) {
    if let RunEvent::Exit = event {
        before_exit(app);
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

/// What a normal quit does last. Also run when the Windows updater ends the
/// process itself (`updates.rs`), which skips `RunEvent::Exit`.
pub fn before_exit<R: Runtime>(app: &AppHandle<R>) {
    // Unload the Whisper model before static destructors run (see EngineSlot::clear).
    if let Some(slot) = app.try_state::<Arc<EngineSlot>>() {
        slot.clear();
    }
    // A quit, not a crash (APP-09).
    if let Some(health) = app.try_state::<Arc<Health>>() {
        health.clean_exit();
    }
}
