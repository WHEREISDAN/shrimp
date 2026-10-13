use std::sync::Arc;

use tauri::State;

use crate::stt::{ModelManager, ModelStatus};

#[tauri::command]
pub fn get_models(models: State<'_, Arc<ModelManager>>) -> Vec<ModelStatus> {
    models.statuses()
}

/// Retries the default model download and load (after a failure).
#[tauri::command]
pub fn download_default_model(models: State<'_, Arc<ModelManager>>) {
    tauri::async_runtime::spawn(Arc::clone(&models).prepare_default());
}
