//! Insights (STA-01..05): stats, the voice profile and the share card.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Local;
use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use super::CommandError;
use super::history::Clipboard;
use crate::insights::profile::{self, ProfileError, VoiceProfileStatus};
use crate::insights::{self, Insights};
use crate::llm::LlmSource;
use crate::settings::SettingsStore;
use crate::storage::{Database, StorageError};

/// The LLM the settings choose, for the voice profile.
pub struct AppLlm(pub Arc<dyn LlmSource>);

/// Share cards are PNGs well under this size.
const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;

async fn read<T: Send + 'static>(
    database: &Arc<Database>,
    work: impl FnOnce(&mut Connection) -> Result<T, StorageError> + Send + 'static,
) -> Result<T, CommandError> {
    let database = Arc::clone(database);
    tauri::async_runtime::spawn_blocking(move || database.with(work))
        .await
        .map_err(|err| CommandError::new("storage", err))?
        .map_err(|err| CommandError::new("storage", err))
}

#[tauri::command]
pub async fn get_insights(database: State<'_, Arc<Database>>) -> Result<Insights, CommandError> {
    read(&database, |db| {
        insights::insights(db, Local::now().date_naive())
    })
    .await
}

#[tauri::command]
pub async fn get_voice_profile(
    database: State<'_, Arc<Database>>,
) -> Result<VoiceProfileStatus, CommandError> {
    read(&database, |db| profile::status(db)).await
}

/// Why a profile couldn't be built. Mirrors `VoiceProfileError` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum VoiceProfileError {
    /// "You only have N words stored locally."
    NotEnoughStored {
        stored: i64,
        needed: i64,
    },
    NotYet {
        next_at: i64,
    },
    Failed {
        message: String,
    },
}

/// Generate profile or Regenerate profile, written in `language` (an
/// English language name such as "German").
#[tauri::command]
pub async fn generate_voice_profile(
    language: String,
    database: State<'_, Arc<Database>>,
    llm: State<'_, AppLlm>,
    settings: State<'_, Arc<SettingsStore>>,
) -> Result<VoiceProfileStatus, VoiceProfileError> {
    let ai = settings.get().effective().ai;
    let generated = profile::generate(
        Arc::clone(&database),
        Arc::clone(&llm.0),
        ai,
        language.chars().take(40).collect(),
    )
    .await;
    if let Err(error) = generated {
        log::warn!("voice profile failed: {error}");
        return Err(match error {
            ProfileError::NotEnoughStored { stored, needed } => {
                VoiceProfileError::NotEnoughStored { stored, needed }
            }
            ProfileError::NotYet(next_at) => VoiceProfileError::NotYet { next_at },
            // The card says "Something went wrong. Try again."; a provider adds why.
            ProfileError::Llm(error) => VoiceProfileError::Failed {
                message: error.to_string(),
            },
            ProfileError::Unreadable | ProfileError::Storage(_) => VoiceProfileError::Failed {
                message: String::new(),
            },
        });
    }
    read(&database, |db| profile::status(db))
        .await
        .map_err(|error| VoiceProfileError::Failed {
            message: error.message,
        })
}

fn png_body(request: &tauri::ipc::Request<'_>) -> Result<Vec<u8>, CommandError> {
    match request.body() {
        tauri::ipc::InvokeBody::Raw(bytes)
            if bytes.starts_with(b"\x89PNG") && bytes.len() <= MAX_IMAGE_BYTES =>
        {
            Ok(bytes.clone())
        }
        _ => Err(CommandError::new("invalid", "Expected a PNG image.")),
    }
}

/// Copy image: the share card onto the clipboard (STA-03).
#[tauri::command]
pub async fn copy_image(
    request: tauri::ipc::Request<'_>,
    clipboard: State<'_, Clipboard>,
) -> Result<(), CommandError> {
    let png = png_body(&request)?;
    let inserter = Arc::clone(&clipboard.0);
    tauri::async_runtime::spawn_blocking(move || inserter.copy_png(&png))
        .await
        .map_err(|err| CommandError::new("platform", err))?
        .map_err(|err| CommandError::new("platform", err))
}

/// Download: the share card into Downloads, named by its title and date.
#[tauri::command]
pub async fn save_image(
    request: tauri::ipc::Request<'_>,
    app: AppHandle,
) -> Result<String, CommandError> {
    let png = png_body(&request)?;
    let title = request
        .headers()
        .get("x-file-name")
        .and_then(|value| value.to_str().ok())
        .map(|name| file_safe(&percent_decode(name)))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Shrimp".into());
    let folder = app
        .path()
        .download_dir()
        .map_err(|err| CommandError::new("platform", err))?;
    let stem = format!("{title} {}", Local::now().format("%Y-%m-%d"));
    let path = unique(&folder, &stem);
    std::fs::write(&path, png).map_err(|err| CommandError::new("io", err))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Undoes `encodeURIComponent`, which keeps the header ASCII.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = bytes
            .get(index + 1..index + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match (bytes[index], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                index += 3;
            }
            (byte, _) => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn file_safe(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '\0') && !c.is_control())
        .take(80)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn unique(folder: &std::path::Path, stem: &str) -> PathBuf {
    let mut path = folder.join(format!("{stem}.png"));
    let mut copy = 2;
    while path.exists() {
        path = folder.join(format!("{stem} ({copy}).png"));
        copy += 1;
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_file_name_header() {
        assert_eq!(percent_decode("Deine%20Aktivit%C3%A4t"), "Deine Aktivität");
        assert_eq!(percent_decode("100%"), "100%");
    }

    #[test]
    fn file_names_lose_path_characters() {
        assert_eq!(
            file_safe("Your Shrimp/activity: 2026"),
            "Your Shrimpactivity 2026"
        );
    }

    #[test]
    fn profile_errors_serialize_with_a_kind() {
        let json = serde_json::to_value(VoiceProfileError::NotEnoughStored {
            stored: 900,
            needed: 2_000,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({"kind": "notEnoughStored", "stored": 900, "needed": 2000})
        );
    }
}
