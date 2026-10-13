//! Shared by the evals: the provider and model from the environment, keys
//! from environment variables, and a median.
#![allow(dead_code)]

use std::sync::Arc;

use shrimp_lib::llm::LlmProviderId;
use shrimp_lib::secrets::{KeyAccount, MemorySecrets, Secrets};
use shrimp_lib::settings::Settings;

/// SHRIMP_EVAL_PROVIDER (ollama by default) and SHRIMP_EVAL_MODEL.
pub fn provider_from_env() -> (LlmProviderId, String) {
    let provider = match std::env::var("SHRIMP_EVAL_PROVIDER").as_deref() {
        Ok("anthropic") => LlmProviderId::Anthropic,
        Ok("openai") => LlmProviderId::OpenAi,
        Ok("groq") => LlmProviderId::Groq,
        _ => LlmProviderId::Ollama,
    };
    let model = std::env::var("SHRIMP_EVAL_MODEL").unwrap_or_default();
    (provider, model)
}

/// Settings that use `provider` with `model`.
pub fn settings_for(provider: LlmProviderId, model: &str) -> Settings {
    let mut settings = Settings::default();
    settings.ai.provider = provider;
    match provider {
        LlmProviderId::Anthropic => settings.ai.anthropic_model = model.into(),
        LlmProviderId::OpenAi => settings.ai.openai_model = model.into(),
        LlmProviderId::Groq => settings.ai.groq_model = model.into(),
        LlmProviderId::Ollama => settings.ai.ollama_model = model.into(),
    }
    settings
}

/// Keys from ANTHROPIC_API_KEY, OPENAI_API_KEY and GROQ_API_KEY, in memory.
pub fn secrets() -> Arc<Secrets> {
    let keys = [
        (KeyAccount::Anthropic, "ANTHROPIC_API_KEY"),
        (KeyAccount::OpenAi, "OPENAI_API_KEY"),
        (KeyAccount::Groq, "GROQ_API_KEY"),
    ]
    .into_iter()
    .filter_map(|(account, variable)| Some((account, std::env::var(variable).ok()?)));
    Arc::new(Secrets::new(Box::new(MemorySecrets::with(keys))))
}

pub fn median(values: &mut [u64]) -> u64 {
    values.sort_unstable();
    values.get(values.len() / 2).copied().unwrap_or_default()
}

pub fn p90(values: &mut [u64]) -> u64 {
    values.sort_unstable();
    values
        .get(values.len() * 9 / 10)
        .copied()
        .unwrap_or_default()
}
