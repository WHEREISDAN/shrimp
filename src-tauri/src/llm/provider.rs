use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::t;

/// Mirrors `LlmProviderId` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LlmProviderId {
    Anthropic,
    #[serde(rename = "openai")]
    OpenAi,
    Groq,
    Ollama,
}

impl LlmProviderId {
    pub fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic",
            Self::OpenAi => "OpenAI",
            Self::Groq => "Groq",
            Self::Ollama => "Ollama",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// A provider-neutral completion request. Sampling settings are left to each
/// provider because models differ in what they accept (Claude Haiku 5.5, for
/// example, rejects `temperature`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmRequest {
    pub system: String,
    pub messages: Vec<Message>,
    pub max_output_tokens: u32,
    /// A different answer is wanted (a transform's Retry). Providers that run
    /// deterministic by default (Ollama) sample more freely.
    pub varied: bool,
}

impl LlmRequest {
    pub fn single(system: String, user: String, max_output_tokens: u32) -> Self {
        Self {
            system,
            messages: vec![Message {
                role: Role::User,
                content: user,
            }],
            max_output_tokens,
            varied: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmResponse {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LlmError {
    #[error("{}", t!("error.llm.missingKey"))]
    MissingKey,
    /// The provider rejected the key (401/403).
    #[error("{}", t!("error.llm.unauthorized", details = .0))]
    Unauthorized(String),
    #[error("{}", t!("error.llm.rateLimited", details = .0))]
    RateLimited(String),
    /// The account is out of credits or over its spend limit.
    #[error("{}", t!("error.llm.quota", details = .0))]
    Quota(String),
    /// No connection to the provider (offline, DNS, refused).
    #[error("{}", t!("error.llm.offline", details = .0))]
    Offline(String),
    #[error("{}", t!("error.llm.timeout"))]
    Timeout,
    /// The provider cannot serve the request now: overloaded, Ollama not
    /// running, model missing.
    #[error("{}", t!("error.llm.unavailable", details = .0))]
    Unavailable(String),
    #[error("{}", t!("error.llm.failed", details = .0))]
    Failed(String),
    #[error("{}", t!("error.llm.cancelled"))]
    Cancelled,
}

impl LlmError {
    /// A short reason for a notice, e.g. "Ollama isn't running".
    pub fn describe(&self, provider: LlmProviderId) -> String {
        let name = provider.label();
        match self {
            Self::MissingKey => t!("error.llm.reason.missingKey", provider = name),
            Self::Unauthorized(_) => t!("error.llm.reason.unauthorized", provider = name),
            Self::RateLimited(_) => t!("error.llm.reason.rateLimited", provider = name),
            Self::Quota(_) => t!("error.llm.reason.quota", provider = name),
            Self::Offline(_) => t!("error.llm.reason.offline", provider = name),
            Self::Timeout => t!("error.llm.reason.timeout", provider = name),
            Self::Unavailable(message) if message.contains(name) => message.clone(),
            Self::Unavailable(_) => t!("error.llm.reason.unavailable", provider = name),
            Self::Failed(message) => message.clone(),
            Self::Cancelled => t!("error.llm.cancelled"),
        }
    }
}

/// One LLM backend, configured with its model when constructed.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn id(&self) -> LlmProviderId;

    fn model(&self) -> &str;

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError>;

    /// Opens the connection (and loads a local model) while the user is still
    /// speaking, so the cleanup request starts warm.
    async fn warm_up(&self) {}
}
