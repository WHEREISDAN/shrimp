//! Picks the speech engine the settings choose for each dictation: local
//! Whisper, or OpenAI or Groq with the user's key.

use std::sync::Arc;

use async_trait::async_trait;

use super::openai_compat::{CloudStt, GROQ_URL, OPENAI_URL};
use super::{EngineSlot, SttEngine, SttEngineId, SttError, SttOptions, Transcript};
use crate::audio::AudioClip;
use crate::llm::catalog;
use crate::secrets::Secrets;
use crate::settings::SettingsStore;

#[derive(Clone)]
pub struct SttRouter {
    local: Arc<EngineSlot>,
    settings: Arc<SettingsStore>,
    secrets: Arc<Secrets>,
    client: reqwest::Client,
}

impl SttRouter {
    pub fn new(
        local: Arc<EngineSlot>,
        settings: Arc<SettingsStore>,
        secrets: Arc<Secrets>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            local,
            settings,
            secrets,
            client,
        }
    }

    /// The engine for the next dictation.
    pub fn selected(&self) -> Result<Arc<dyn SttEngine>, SttError> {
        let stt = self.settings.get().stt;
        let (url, model) = match stt.engine {
            SttEngineId::LocalWhisper => return Ok(Arc::clone(&self.local) as Arc<dyn SttEngine>),
            SttEngineId::OpenAi => (OPENAI_URL, stt.openai_model),
            SttEngineId::Groq => (GROQ_URL, stt.groq_model),
        };
        let account = catalog::stt_key_account(stt.engine).ok_or(SttError::Cancelled)?;
        let key = self.secrets.get(account).ok_or_else(|| {
            SttError::Unavailable(format!(
                "Add your {} API key in Settings to transcribe with {}.",
                account.label(),
                stt.engine.label()
            ))
        })?;
        Ok(Arc::new(CloudStt::new(
            stt.engine,
            self.client.clone(),
            url,
            key,
            model,
        )))
    }
}

#[async_trait]
impl SttEngine for SttRouter {
    fn id(&self) -> SttEngineId {
        self.settings.get().stt.engine
    }

    async fn transcribe(
        &self,
        clip: &AudioClip,
        options: &SttOptions,
    ) -> Result<Transcript, SttError> {
        // Reading a key can block on the keychain.
        let router = self.clone();
        let engine = tokio::task::spawn_blocking(move || router.selected())
            .await
            .map_err(|err| SttError::Failed(err.to_string()))??;
        engine.transcribe(clip, options).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::{KeyAccount, MemorySecrets};
    use crate::settings::Settings;

    #[test]
    fn a_cloud_engine_needs_its_key() {
        let mut settings = Settings::default();
        settings.stt.engine = SttEngineId::Groq;
        let secrets = Arc::new(Secrets::new(Box::new(MemorySecrets::default())));
        let router = SttRouter::new(
            Arc::new(EngineSlot::new("loading")),
            Arc::new(SettingsStore::in_memory(settings)),
            Arc::clone(&secrets),
            reqwest::Client::new(),
        );
        let error = router.selected().err().unwrap();
        assert_eq!(
            error.to_string(),
            "speech-to-text unavailable: Add your Groq API key in Settings to transcribe with Groq."
        );
        secrets.set(KeyAccount::Groq, "gsk_test").unwrap();
        assert_eq!(router.selected().unwrap().id(), SttEngineId::Groq);
        assert_eq!(router.id(), SttEngineId::Groq);
    }
}
