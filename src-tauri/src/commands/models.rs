use std::sync::Arc;

use tauri::State;

use super::CommandError;
use crate::settings::SettingsStore;
use crate::stt::{ModelManager, ModelStatus};

#[tauri::command]
pub fn get_models(models: State<'_, Arc<ModelManager>>) -> Vec<ModelStatus> {
    models.statuses()
}

/// Retries getting the chosen model ready (after a failure).
#[tauri::command]
pub fn retry_model(models: State<'_, Arc<ModelManager>>, settings: State<'_, Arc<SettingsStore>>) {
    let chosen = settings.get().stt.local_model;
    tauri::async_runtime::spawn(Arc::clone(&models).prepare(chosen));
}

/// Downloads a model without switching to it; progress arrives as `models://status`.
#[tauri::command]
pub async fn download_model(
    id: String,
    models: State<'_, Arc<ModelManager>>,
) -> Result<(), CommandError> {
    Arc::clone(&models)
        .download(id)
        .await
        .map_err(|message| CommandError::new("download", message))
}

/// Deletes a downloaded model that is not the one in use (ENG-08).
#[tauri::command]
pub fn delete_model(
    id: String,
    models: State<'_, Arc<ModelManager>>,
    settings: State<'_, Arc<SettingsStore>>,
) -> Result<(), CommandError> {
    models
        .delete(&id, &settings.get().stt.local_model)
        .map_err(|message| CommandError::new("invalid", message))
}
