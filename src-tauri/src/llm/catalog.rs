//! The models Settings offers per provider and speech engine, and the
//! defaults. Latency measurements behind the defaults are in
//! docs/ARCHITECTURE.md ("LLM providers and the edit pipeline").

use serde::Serialize;

use super::LlmProviderId;
use crate::secrets::KeyAccount;
use crate::stt::SttEngineId;

pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
pub const DEFAULT_OPENAI_STT_MODEL: &str = "gpt-transcribe";
pub const DEFAULT_GROQ_STT_MODEL: &str = "whisper-large-v3-turbo";

/// Installed Ollama models Shrimp prefers, best first, when none is chosen.
/// Measured on the eval set (docs/ARCHITECTURE.md): Qwen 3.5 4B matched
/// Gemma 3 4B's accuracy at under half its latency; Llama 3.2 3B is fastest
/// but drops more meaning.
pub const OLLAMA_PREFERENCE: [&str; 3] = ["qwen3.5:4b", "gemma3:4b", "llama3.2:3b"];

/// Mirrors `ModelOption` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    pub id: &'static str,
    pub label: &'static str,
    pub note: &'static str,
}

const fn model(id: &'static str, label: &'static str, note: &'static str) -> ModelOption {
    ModelOption { id, label, note }
}

const ANTHROPIC: &[ModelOption] = &[
    model(
        "claude-haiku-5-5",
        "Claude Haiku 5.5",
        "Fastest. Recommended.",
    ),
    model(
        "claude-sonnet-5-5",
        "Claude Sonnet 5.5",
        "Slower, more careful.",
    ),
    model(
        "claude-opus-5-5",
        "Claude Opus 5.5",
        "Slowest; always thinks first.",
    ),
];

const OPENAI: &[ModelOption] = &[
    model("gpt-6-luna", "GPT-6 Luna", "Fastest. Recommended."),
    model("gpt-5.6-luna", "GPT-5.6 Luna", "Previous generation."),
    model("gpt-5.4-mini", "GPT-5.4 mini", "Larger, slower."),
];

const GROQ: &[ModelOption] = &[
    model("openai/gpt-oss-20b", "GPT-OSS 20B", "Fastest. Recommended."),
    model("openai/gpt-oss-120b", "GPT-OSS 120B", "Larger, slower."),
    model("qwen/qwen3.8-27b", "Qwen 3.8 27B", "Preview model."),
];

const OPENAI_STT: &[ModelOption] = &[
    model("gpt-transcribe", "GPT Transcribe", "Recommended."),
    model(
        "gpt-4o-mini-transcribe",
        "GPT-4o mini Transcribe",
        "Retired by OpenAI in February 2027.",
    ),
];

const GROQ_STT: &[ModelOption] = &[
    model(
        "whisper-large-v3-turbo",
        "Whisper Large v3 Turbo",
        "Fastest. Recommended.",
    ),
    model(
        "whisper-large-v3",
        "Whisper Large v3",
        "Slower, slightly more accurate.",
    ),
];

pub fn models(provider: LlmProviderId) -> &'static [ModelOption] {
    match provider {
        LlmProviderId::Anthropic => ANTHROPIC,
        LlmProviderId::OpenAi => OPENAI,
        LlmProviderId::Groq => GROQ,
        LlmProviderId::Ollama => &[],
    }
}

pub fn default_model(provider: LlmProviderId) -> &'static str {
    models(provider).first().map_or("", |model| model.id)
}

pub fn key_account(provider: LlmProviderId) -> Option<KeyAccount> {
    match provider {
        LlmProviderId::Anthropic => Some(KeyAccount::Anthropic),
        LlmProviderId::OpenAi => Some(KeyAccount::OpenAi),
        LlmProviderId::Groq => Some(KeyAccount::Groq),
        LlmProviderId::Ollama => None,
    }
}

pub fn stt_key_account(engine: SttEngineId) -> Option<KeyAccount> {
    match engine {
        SttEngineId::LocalWhisper => None,
        SttEngineId::OpenAi => Some(KeyAccount::OpenAi),
        SttEngineId::Groq => Some(KeyAccount::Groq),
    }
}

/// Mirrors `ProviderInfo` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: LlmProviderId,
    pub label: &'static str,
    pub models: &'static [ModelOption],
    pub default_model: &'static str,
    pub key: Option<KeyAccount>,
}

/// Mirrors `SttEngineInfo` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SttEngineInfo {
    pub id: SttEngineId,
    pub label: &'static str,
    pub models: &'static [ModelOption],
    pub default_model: &'static str,
    pub key: Option<KeyAccount>,
}

/// Mirrors `AiOptions` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiOptions {
    pub providers: Vec<ProviderInfo>,
    pub stt_engines: Vec<SttEngineInfo>,
}

pub fn options() -> AiOptions {
    let providers = [
        LlmProviderId::Anthropic,
        LlmProviderId::OpenAi,
        LlmProviderId::Groq,
        LlmProviderId::Ollama,
    ]
    .into_iter()
    .map(|id| ProviderInfo {
        id,
        label: id.label(),
        models: models(id),
        default_model: default_model(id),
        key: key_account(id),
    })
    .collect();
    let stt_engines = vec![
        SttEngineInfo {
            id: SttEngineId::LocalWhisper,
            label: "Whisper on this Mac",
            models: &[],
            default_model: "",
            key: None,
        },
        SttEngineInfo {
            id: SttEngineId::OpenAi,
            label: "OpenAI",
            models: OPENAI_STT,
            default_model: DEFAULT_OPENAI_STT_MODEL,
            key: Some(KeyAccount::OpenAi),
        },
        SttEngineInfo {
            id: SttEngineId::Groq,
            label: "Groq",
            models: GROQ_STT,
            default_model: DEFAULT_GROQ_STT_MODEL,
            key: Some(KeyAccount::Groq),
        },
    ];
    AiOptions {
        providers,
        stt_engines,
    }
}

/// Picks the Ollama model: the configured one, else the first preferred
/// model that is installed, else the first installed.
pub fn pick_ollama_model(configured: &str, installed: &[String]) -> Option<String> {
    if !configured.trim().is_empty() {
        return Some(configured.trim().to_owned());
    }
    OLLAMA_PREFERENCE
        .iter()
        .find_map(|preferred| {
            installed.iter().find(|name| {
                name.as_str() == *preferred || name.as_str() == format!("{preferred}:latest")
            })
        })
        .or_else(|| installed.first())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cloud_provider_has_a_default_and_a_key() {
        for provider in [
            LlmProviderId::Anthropic,
            LlmProviderId::OpenAi,
            LlmProviderId::Groq,
        ] {
            assert!(!default_model(provider).is_empty());
            assert!(key_account(provider).is_some());
        }
        assert_eq!(key_account(LlmProviderId::Ollama), None);
    }

    #[test]
    fn ollama_prefers_known_models_then_the_first_installed() {
        let installed = vec!["qwen2.5-coder:7b".to_owned(), "llama3.2:3b".to_owned()];
        assert_eq!(pick_ollama_model("", &installed).unwrap(), "llama3.2:3b");
        let installed = vec!["llama3.2:3b".to_owned(), "qwen3.5:4b".to_owned()];
        assert_eq!(pick_ollama_model("", &installed).unwrap(), "qwen3.5:4b");
        assert_eq!(
            pick_ollama_model("", &["mistral:7b".to_owned()]).unwrap(),
            "mistral:7b"
        );
        assert_eq!(pick_ollama_model(" phi4 ", &installed).unwrap(), "phi4");
        assert_eq!(pick_ollama_model("", &[]), None);
    }

    #[test]
    fn options_serialize_for_the_settings_page() {
        let json = serde_json::to_value(options()).unwrap();
        assert_eq!(json["providers"][0]["id"], "anthropic");
        assert_eq!(json["providers"][1]["key"], "openai");
        assert_eq!(json["sttEngines"][0]["id"], "local-whisper");
        assert_eq!(json["sttEngines"][1]["defaultModel"], "gpt-transcribe");
    }
}
