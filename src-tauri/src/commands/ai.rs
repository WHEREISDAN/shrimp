//! AI provider settings: model catalog, API keys, Ollama models, and a test
//! run of the cleanup.

use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use tauri::State;

use super::CommandError;
use crate::edit::{Editor, RAW_INSERTED};
use crate::llm::catalog::{self, AiOptions};
use crate::llm::{LlmError, ollama};
use crate::platform::AppIdentity;
use crate::secrets::{KeyAccount, SecretError, Secrets};
use crate::settings::SettingsStore;

/// The shared HTTP client, in managed state.
pub struct HttpClient(pub reqwest::Client);

const TEST_DICTATION: &str = "um so this is uh a quick test of shrimp no wait of my AI provider";

/// Which keys are stored. Mirrors `ApiKeys` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ApiKeys {
    pub anthropic: bool,
    pub openai: bool,
    pub groq: bool,
}

fn api_keys(secrets: &Secrets) -> ApiKeys {
    ApiKeys {
        anthropic: secrets.has(KeyAccount::Anthropic),
        openai: secrets.has(KeyAccount::OpenAi),
        groq: secrets.has(KeyAccount::Groq),
    }
}

fn keychain_error(error: SecretError) -> CommandError {
    match error {
        SecretError::Invalid(message) => CommandError::new("invalid", message),
        SecretError::Keychain(message) => CommandError::new("keychain", message),
    }
}

#[tauri::command]
pub fn get_ai_options() -> AiOptions {
    catalog::options()
}

#[tauri::command]
pub async fn get_api_keys(secrets: State<'_, Arc<Secrets>>) -> Result<ApiKeys, CommandError> {
    let secrets = Arc::clone(&secrets);
    tauri::async_runtime::spawn_blocking(move || api_keys(&secrets))
        .await
        .map_err(|err| CommandError::new("keychain", err))
}

/// Stores a key in the keychain. The key never comes back to the webview.
#[tauri::command]
pub async fn set_api_key(
    account: KeyAccount,
    key: String,
    secrets: State<'_, Arc<Secrets>>,
) -> Result<ApiKeys, CommandError> {
    let secrets = Arc::clone(&secrets);
    tauri::async_runtime::spawn_blocking(move || {
        secrets.set(account, &key).map_err(keychain_error)?;
        Ok(api_keys(&secrets))
    })
    .await
    .map_err(|err| CommandError::new("keychain", err))?
}

#[tauri::command]
pub async fn delete_api_key(
    account: KeyAccount,
    secrets: State<'_, Arc<Secrets>>,
) -> Result<ApiKeys, CommandError> {
    let secrets = Arc::clone(&secrets);
    tauri::async_runtime::spawn_blocking(move || {
        secrets.delete(account).map_err(keychain_error)?;
        Ok(api_keys(&secrets))
    })
    .await
    .map_err(|err| CommandError::new("keychain", err))?
}

/// The models installed in Ollama at the configured address.
#[tauri::command]
pub async fn list_ollama_models(
    settings: State<'_, Arc<SettingsStore>>,
    client: State<'_, HttpClient>,
) -> Result<Vec<String>, CommandError> {
    let url = settings.get().ai.ollama_url;
    ollama::installed_models(&client.0, &url)
        .await
        .map_err(|error| match error {
            LlmError::Unavailable(message) => CommandError::new("unavailable", message),
            error => CommandError::new("provider", error),
        })
}

/// The result of a test cleanup. Mirrors `AiTestResult` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTestResult {
    pub ok: bool,
    pub latency_ms: u64,
    pub input: &'static str,
    pub output: String,
    pub message: Option<String>,
}

/// Cleans up a sample dictation with the current settings.
#[tauri::command]
pub async fn test_ai_provider(
    editor: State<'_, Arc<Editor>>,
) -> Result<AiTestResult, CommandError> {
    let started = Instant::now();
    let app = AppIdentity {
        bundle_id: "com.shrimp.test".into(),
        name: "Shrimp".into(),
    };
    let edited = editor.edit(TEST_DICTATION, Some(&app)).await;
    Ok(AiTestResult {
        ok: edited.formatted,
        latency_ms: edited
            .llm_ms
            .unwrap_or_else(|| started.elapsed().as_millis() as u64),
        input: TEST_DICTATION,
        output: edited.text.plain,
        // Nothing was inserted, so the notice's last sentence doesn't apply.
        message: edited
            .notice
            .map(|notice| notice.message.replace(&format!(" {RAW_INSERTED}"), "")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_status_serializes_by_provider() {
        let keys = ApiKeys {
            anthropic: true,
            openai: false,
            groq: false,
        };
        assert_eq!(
            serde_json::to_value(keys).unwrap(),
            serde_json::json!({"anthropic": true, "openai": false, "groq": false})
        );
    }
}
