//! Debug-only commands for agents driving the dev app through tauri-mcp.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::CommandError;
use crate::audio::AudioSourceSpec;
use crate::dictation::ControllerHandle;
use crate::edit::{AppCategory, Editor, Target, style};
use crate::platform::AppIdentity;

/// The current audio source, as `mic`, `mic:<device>` or `wav:<path>`.
#[tauri::command]
pub fn dev_get_audio_source(controller: State<'_, ControllerHandle>) -> String {
    controller.audio_source().to_string()
}

/// Switches the audio source for the next dictation, e.g. `wav:/abs/path.wav`.
#[tauri::command]
pub fn dev_set_audio_source(
    spec: String,
    controller: State<'_, ControllerHandle>,
) -> Result<String, CommandError> {
    let spec: AudioSourceSpec = spec
        .parse()
        .map_err(|err| CommandError::new("invalidSpec", err))?;
    if let AudioSourceSpec::WavFile(path) = &spec
        && !path.is_file()
    {
        return Err(CommandError::new(
            "notFound",
            format!("{} does not exist", path.display()),
        ));
    }
    controller.set_audio_source(spec.clone());
    Ok(spec.to_string())
}

/// A category forced for one app in end-to-end tests; `ai` marks an AI app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DevCategory {
    Personal,
    Work,
    Email,
    Other,
    Ai,
}

/// Treats `app_id` (a bundle ID) as `category` until reset with `null`.
#[tauri::command]
pub fn dev_set_app_category(app_id: String, category: Option<DevCategory>) {
    let target = category.map(|category| match category {
        DevCategory::Ai => Target {
            category: AppCategory::Other,
            ai_app: true,
        },
        other => Target {
            category: match other {
                DevCategory::Personal => AppCategory::Personal,
                DevCategory::Work => AppCategory::Work,
                DevCategory::Email => AppCategory::Email,
                _ => AppCategory::Other,
            },
            ai_app: false,
        },
    });
    style::set_override(&app_id, target);
}

/// The edit pipeline's result for a transcript, without STT or pasting.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevEdited {
    pub plain: String,
    pub html: Option<String>,
    pub formatted: bool,
    pub notice: Option<String>,
    pub llm_ms: Option<u64>,
}

#[tauri::command]
pub async fn dev_edit_text(
    text: String,
    app_id: Option<String>,
    editor: State<'_, Arc<Editor>>,
) -> Result<DevEdited, CommandError> {
    let app = app_id.map(|bundle_id| AppIdentity {
        bundle_id,
        name: String::new(),
    });
    let edited = editor.edit(&text, app.as_ref()).await;
    Ok(DevEdited {
        plain: edited.text.plain,
        html: edited.text.html,
        formatted: edited.formatted,
        notice: edited.notice.map(|notice| notice.message),
        llm_ms: edited.llm_ms,
    })
}
