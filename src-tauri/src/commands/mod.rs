//! Tauri IPC commands. Keep these thin: parse input, call into a domain
//! module, map errors. Register new commands in `lib.rs` and in `NAMES`.

pub mod ai;
pub mod app;
#[cfg(debug_assertions)]
pub mod dev;
pub mod dictation;
pub mod history;
pub mod insights;
pub mod languages;
pub mod library;
pub mod microphones;
pub mod models;
pub mod permissions;
pub mod settings;
pub mod shortcuts;
pub mod system;

use serde::Serialize;

/// Every registered command, advertised to the dev MCP bridge.
#[cfg_attr(not(debug_assertions), allow(dead_code))]
pub const NAMES: &[&str] = &[
    "app_info",
    "own_field_focused",
    "get_dictation_state",
    "flow_bar_control",
    "flow_bar_ready",
    "flow_bar_layout",
    "flow_bar_drag",
    "flow_bar_action",
    "run_notice_action",
    "transform_action",
    "get_shortcuts",
    "record_shortcut",
    "cancel_shortcut_recording",
    "remove_shortcut",
    "reset_shortcuts",
    "get_languages",
    "set_languages",
    "set_active_language",
    "get_microphones",
    "select_microphone",
    "rank_microphone",
    "forget_microphone",
    "start_mic_preview",
    "stop_mic_preview",
    "get_models",
    "retry_model",
    "download_model",
    "delete_model",
    "get_permissions",
    "open_permission_settings",
    "get_settings",
    "update_settings",
    "get_ai_options",
    "get_api_keys",
    "set_api_key",
    "delete_api_key",
    "list_ollama_models",
    "test_ai_provider",
    "list_dictionary",
    "add_dictionary_entry",
    "update_dictionary_entry",
    "set_dictionary_pinned",
    "delete_dictionary_entries",
    "preview_dictionary_import",
    "import_dictionary",
    "list_snippets",
    "add_snippet",
    "update_snippet",
    "delete_snippets",
    "preview_snippet_import",
    "import_snippets",
    "list_history",
    "copy_history",
    "set_history_raw",
    "retry_history",
    "history_audio",
    "extract_history_audio",
    "delete_history",
    "delete_all_history",
    "set_history_storage",
    "get_insights",
    "get_voice_profile",
    "generate_voice_profile",
    "copy_image",
    "save_image",
    "get_app_health",
    "dismiss_crash",
    "check_storage",
    "open_storage_settings",
    "repair_data",
    "reset_app",
    #[cfg(debug_assertions)]
    "dev_get_audio_source",
    #[cfg(debug_assertions)]
    "dev_set_audio_source",
    #[cfg(debug_assertions)]
    "dev_set_app_category",
    #[cfg(debug_assertions)]
    "dev_edit_text",
    #[cfg(debug_assertions)]
    "dev_read_context",
    #[cfg(debug_assertions)]
    "dev_set_microphones",
    #[cfg(debug_assertions)]
    "dev_set_permissions",
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
