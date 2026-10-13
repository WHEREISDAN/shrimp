//! Tauri IPC commands. Keep these thin: parse input, call into a domain
//! module, map errors. Register new commands in `lib.rs` and in `NAMES`.

pub mod app;
#[cfg(debug_assertions)]
pub mod dev;
pub mod dictation;
pub mod models;
pub mod permissions;

use serde::Serialize;

/// Every registered command, advertised to the dev MCP bridge.
#[cfg_attr(not(debug_assertions), allow(dead_code))]
pub const NAMES: &[&str] = &[
    "app_info",
    "get_dictation_state",
    "flow_bar_control",
    "flow_bar_ready",
    "flow_bar_layout",
    "run_notice_action",
    "get_shortcuts",
    "get_models",
    "download_default_model",
    "get_permissions",
    "open_permission_settings",
    #[cfg(debug_assertions)]
    "dev_get_audio_source",
    #[cfg(debug_assertions)]
    "dev_set_audio_source",
];

/// The error every command returns. Mirrors `CommandError` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
}

impl CommandError {
    pub fn new(code: &'static str, message: impl ToString) -> Self {
        Self {
            code,
            message: message.to_string(),
        }
    }
}
