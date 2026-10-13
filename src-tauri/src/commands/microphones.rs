//! Settings › Microphone and the Flow Bar's microphone menu (MIC-02..06).

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use super::CommandError;
use crate::app::events::MIC_LEVEL_EVENT;
use crate::mic::{MicrophoneList, Microphones};

fn invalid(message: String) -> CommandError {
    CommandError::new("invalid", message)
}

#[tauri::command]
pub fn get_microphones(microphones: State<'_, Arc<Microphones>>) -> MicrophoneList {
    microphones.list()
}

/// Chooses a microphone by id, or Auto-detect with `null`.
#[tauri::command]
pub fn select_microphone(
    id: Option<String>,
    microphones: State<'_, Arc<Microphones>>,
) -> Result<MicrophoneList, CommandError> {
    microphones.select(id).map_err(invalid)?;
    Ok(microphones.list())
}

/// Moves a microphone one place up or down the ranking.
#[tauri::command]
pub fn rank_microphone(
    id: String,
    up: bool,
    microphones: State<'_, Arc<Microphones>>,
) -> Result<MicrophoneList, CommandError> {
    microphones.rank(&id, up).map_err(invalid)?;
    Ok(microphones.list())
}

#[tauri::command]
pub fn forget_microphone(
    id: String,
    microphones: State<'_, Arc<Microphones>>,
) -> Result<MicrophoneList, CommandError> {
    microphones.forget(&id).map_err(invalid)?;
    Ok(microphones.list())
}

/// Starts the live level meter; levels arrive as `mic://level`.
#[tauri::command]
pub fn start_mic_preview(app: AppHandle, microphones: State<'_, Arc<Microphones>>) {
    microphones.start_preview(Box::new(move |level| {
        let _ = app.emit(MIC_LEVEL_EVENT, level);
    }));
}

#[tauri::command]
pub fn stop_mic_preview(microphones: State<'_, Arc<Microphones>>) {
    microphones.stop_preview();
}
