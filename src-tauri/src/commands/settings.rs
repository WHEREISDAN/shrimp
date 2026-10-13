use std::sync::Arc;

use serde_json::Value;
use tauri::State;

use super::CommandError;
use crate::settings::{Settings, SettingsError, SettingsStore};

#[tauri::command]
pub fn get_settings(settings: State<'_, Arc<SettingsStore>>) -> Settings {
    settings.get()
}

/// Applies a JSON merge patch, e.g. `{"ai": {"cleanup": "medium"}}`, and
/// returns the saved settings. Also emitted as `settings://changed`.
#[tauri::command]
pub fn update_settings(
    patch: Value,
    settings: State<'_, Arc<SettingsStore>>,
) -> Result<Settings, CommandError> {
    settings.update(&patch).map_err(settings_error)
}

fn settings_error(error: SettingsError) -> CommandError {
    match error {
        SettingsError::Invalid(message) => CommandError::new("invalid", message),
        SettingsError::Io(error) => CommandError::new("io", error),
    }
}
