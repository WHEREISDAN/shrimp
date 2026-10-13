//! Builds the provider the settings choose, with its key from the keychain.

use std::sync::Arc;

use async_trait::async_trait;

use super::{
    Anthropic, Groq, LlmError, LlmProvider, LlmProviderId, Ollama, OpenAi, anthropic, catalog,
    groq, ollama, openai,
};
use crate::secrets::Secrets;
use crate::settings::AiSettings;
use crate::t;

/// Where the edit pipeline gets its provider; tests substitute fakes.
#[async_trait]
pub trait LlmSource: Send + Sync {
    async fn provider(&self, settings: &AiSettings) -> Result<Arc<dyn LlmProvider>, LlmError>;
}

pub struct Providers {
    client: reqwest::Client,
    secrets: Arc<Secrets>,
}

impl Providers {
    pub fn new(client: reqwest::Client, secrets: Arc<Secrets>) -> Self {
        Self { client, secrets }
    }
}

#[async_trait]
impl LlmSource for Providers {
    async fn provider(&self, settings: &AiSettings) -> Result<Arc<dyn LlmProvider>, LlmError> {
        let provider = settings.provider;
        let client = self.client.clone();
        if provider == LlmProviderId::Ollama {
            let installed = ollama::installed_models(&client, &settings.ollama_url).await?;
            let model = catalog::pick_ollama_model(&settings.ollama_model, &installed)
                .ok_or_else(|| LlmError::Unavailable(t!("error.llm.noOllamaModel")))?;
            return Ok(Arc::new(Ollama::new(client, &settings.ollama_url, model)));
        }
        let account = catalog::key_account(provider).ok_or(LlmError::MissingKey)?;
        // Reading a key can block on the keychain.
        let secrets = Arc::clone(&self.secrets);
        let key = tokio::task::spawn_blocking(move || secrets.get(account))
            .await
            .ok()
            .flatten()
            .ok_or(LlmError::MissingKey)?;
        let model = settings.model(provider).to_owned();
        let built: Arc<dyn LlmProvider> = match provider {
            LlmProviderId::Anthropic => {
                Arc::new(Anthropic::new(client, anthropic::BASE_URL, key, model))
            }
            LlmProviderId::OpenAi => Arc::new(OpenAi::new(client, openai::BASE_URL, key, model)),
            LlmProviderId::Groq => Arc::new(Groq::new(client, groq::BASE_URL, key, model)),
            LlmProviderId::Ollama => unreachable!("handled above"),
        };
        Ok(built)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::{KeyAccount, MemorySecrets};

    #[tokio::test]
    async fn a_cloud_provider_needs_its_key() {
        let secrets = Arc::new(Secrets::new(Box::new(MemorySecrets::default())));
        let providers = Providers::new(reqwest::Client::new(), Arc::clone(&secrets));
        let settings = AiSettings::default();
        assert_eq!(
            providers.provider(&settings).await.err(),
            Some(LlmError::MissingKey)
        );
        secrets.set(KeyAccount::Anthropic, "sk-ant-test").unwrap();
        let provider = providers.provider(&settings).await.unwrap();
        assert_eq!(provider.id(), LlmProviderId::Anthropic);
        assert_eq!(
            provider.model(),
            catalog::default_model(LlmProviderId::Anthropic)
        );
    }

    #[tokio::test]
    async fn ollama_reports_when_it_is_not_running() {
        let secrets = Arc::new(Secrets::new(Box::new(MemorySecrets::default())));
        let providers = Providers::new(reqwest::Client::new(), secrets);
        let settings = AiSettings {
            provider: LlmProviderId::Ollama,
            ollama_url: "http://127.0.0.1:9".into(),
            ..AiSettings::default()
        };
        assert_eq!(
            providers.provider(&settings).await.err(),
            Some(LlmError::Unavailable("Ollama isn't running".into()))
        );
    }
}
