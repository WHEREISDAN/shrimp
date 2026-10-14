use std::sync::Arc;

use serde_json::Value;
use tauri::State;

use super::CommandError;
use crate::app::window::PreviousApp;
use crate::settings::{KnownApp, Settings, SettingsError, SettingsStore, app_formats};
use crate::storage::{Database, stats};

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

/// The apps Style › Formatting per app offers (FMT-15): the app in front
/// before Shrimp's window opened, then the apps dictated into.
#[tauri::command]
pub async fn get_format_apps(
    database: State<'_, Arc<Database>>,
    previous: State<'_, PreviousApp>,
) -> Result<Vec<KnownApp>, CommandError> {
    let database = Arc::clone(&database);
    let dictated =
        tauri::async_runtime::spawn_blocking(move || database.with(|db| stats::apps(db)))
            .await
            .map_err(|err| CommandError::new("storage", err))?
            .map_err(|err| CommandError::new("storage", err))?;
    Ok(app_formats::candidates(
        previous.get(),
        dictated.into_iter().map(|app| (app.app_id, app.name)),
        previous.own_ids(),
    ))
}
