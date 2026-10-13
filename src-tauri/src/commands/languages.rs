//! Settings › Languages and the Flow Bar's language switch (LANG-01..05).

use std::sync::Arc;

use serde::Serialize;
use serde_json::json;
use tauri::State;

use super::CommandError;
use crate::lang::catalog::{self, LanguageOption};
use crate::settings::{LanguageSettings, SettingsStore};

/// Mirrors `Languages` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Languages {
    pub options: Vec<LanguageOption>,
    pub chosen: LanguageSettings,
}

fn invalid(error: impl ToString) -> CommandError {
    CommandError::new("invalid", error)
}

#[tauri::command]
pub fn get_languages(settings: State<'_, Arc<SettingsStore>>) -> Languages {
    Languages {
        options: catalog::languages(),
        chosen: settings.get().languages,
    }
}

/// Saves the Languages dialog: auto-detect or at least one language.
#[tauri::command]
pub fn set_languages(
    auto_detect: bool,
    selected: Vec<String>,
    settings: State<'_, Arc<SettingsStore>>,
) -> Result<LanguageSettings, CommandError> {
    let active = settings
        .get()
        .languages
        .active
        .filter(|active| selected.contains(active));
    let choice = LanguageSettings {
        auto_detect,
        selected,
        active,
    };
    choice.validate_choice().map_err(invalid)?;
    settings
        .update(&json!({"languages": choice}))
        .map(|saved| saved.languages)
        .map_err(invalid)
}

/// The Flow Bar's switch: one of the user's languages, or `null` to detect
/// among them (LANG-05).
#[tauri::command]
pub fn set_active_language(
    id: Option<String>,
    settings: State<'_, Arc<SettingsStore>>,
) -> Result<LanguageSettings, CommandError> {
    settings
        .update(&json!({"languages": {"active": id}}))
        .map(|saved| saved.languages)
        .map_err(invalid)
}
