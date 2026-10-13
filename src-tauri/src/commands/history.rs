//! Home's transcript history (HIS-01..07) and Undo/Redo AI edit (FMT-12).

use std::sync::Arc;

use tauri::{AppHandle, Manager, State};

use super::CommandError;
use crate::dictation::{ControllerHandle, ControllerInput};
use crate::history::{History, HistoryError, HistoryItem, HistoryList};
use crate::platform::{InsertText, TextInserter};
use crate::settings::{HistoryStorage, Settings, SettingsStore};

/// The clipboard, for copying transcripts.
pub struct Clipboard(pub Arc<dyn TextInserter>);

/// What "Never store data locally" asks the user to type.
const DELETE_ALL: &str = "delete all";

fn history_error(error: HistoryError) -> CommandError {
    let code = match error {
        HistoryError::NotFound => "notFound",
        HistoryError::RetryAudioGone
        | HistoryError::PlayAudioGone
        | HistoryError::DownloadAudioGone
        | HistoryError::NotRetryable => "unavailable",
        HistoryError::Storage(_) | HistoryError::Io(_) => "storage",
    };
    CommandError::new(code, error)
}

async fn run<T: Send + 'static>(
    history: &Arc<History>,
    work: impl FnOnce(&History) -> Result<T, HistoryError> + Send + 'static,
) -> Result<T, CommandError> {
    let history = Arc::clone(history);
    tauri::async_runtime::spawn_blocking(move || work(&history))
        .await
        .map_err(|err| CommandError::new("storage", err))?
        .map_err(history_error)
}

async fn copy(clipboard: &Clipboard, text: String) -> Result<(), CommandError> {
    let inserter = Arc::clone(&clipboard.0);
    tauri::async_runtime::spawn_blocking(move || inserter.copy(&InsertText::plain(text)))
        .await
        .map_err(|err| CommandError::new("platform", err))?
        .map_err(|err| CommandError::new("platform", err))
}

#[tauri::command]
pub async fn list_history(
    query: String,
    history: State<'_, Arc<History>>,
) -> Result<HistoryList, CommandError> {
    run(&history, move |history| history.list(&query)).await
}

/// Copies the text a row shows (HIS-01).
#[tauri::command]
pub async fn copy_history(
    id: i64,
    history: State<'_, Arc<History>>,
    clipboard: State<'_, Clipboard>,
) -> Result<(), CommandError> {
    let item = run(&history, move |history| history.get(id)).await?;
    copy(&clipboard, item.entry.shown_text().to_owned()).await
}

/// Undo AI edit (`raw`: shows and copies the raw text) or Redo AI edit (FMT-12).
#[tauri::command]
pub async fn set_history_raw(
    id: i64,
    raw: bool,
    history: State<'_, Arc<History>>,
    clipboard: State<'_, Clipboard>,
) -> Result<HistoryItem, CommandError> {
    let item = run(&history, move |history| history.show_raw(id, raw)).await?;
    if raw {
        copy(&clipboard, item.entry.raw_text.clone()).await?;
    }
    Ok(item)
}

/// Retry or Recover: transcribes the entry's audio again (HIS-05, HIS-06).
#[tauri::command]
pub fn retry_history(id: i64, controller: State<'_, ControllerHandle>) {
    controller.send(ControllerInput::RetryHistory(id));
}

/// The recording as WAV bytes, for Play audio.
#[tauri::command]
pub async fn history_audio(
    id: i64,
    history: State<'_, Arc<History>>,
) -> Result<tauri::ipc::Response, CommandError> {
    let bytes = run(&history, move |history| history.audio(id)).await?;
    Ok(tauri::ipc::Response::new(bytes))
}

/// Extract audio: saves the recording to Downloads and returns its path.
#[tauri::command]
pub async fn extract_history_audio(
    id: i64,
    app: AppHandle,
    history: State<'_, Arc<History>>,
) -> Result<String, CommandError> {
    let folder = app
        .path()
        .download_dir()
        .map_err(|err| CommandError::new("platform", err))?;
    let path = run(&history, move |history| history.extract(id, &folder)).await?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn delete_history(id: i64, history: State<'_, Arc<History>>) -> Result<(), CommandError> {
    run(&history, move |history| history.delete(id)).await
}

/// Data and privacy › Local data storage (HIS-07). "Never store" deletes
/// everything, so it needs `confirmation` to be "delete all".
#[tauri::command]
pub fn set_history_storage(
    storage: HistoryStorage,
    confirmation: Option<String>,
    settings: State<'_, Arc<SettingsStore>>,
) -> Result<Settings, CommandError> {
    let confirmed = confirmation
        .as_deref()
        .is_some_and(|typed| typed.trim().eq_ignore_ascii_case(DELETE_ALL));
    if storage == HistoryStorage::Never && !confirmed {
        return Err(CommandError::new(
            "invalid",
            format!("Type \"{DELETE_ALL}\" to confirm."),
        ));
    }
    settings
        .update(&serde_json::json!({"privacy": {"history": storage}}))
        .map_err(|err| CommandError::new("invalid", err))
}
