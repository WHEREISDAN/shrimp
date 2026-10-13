use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmResponse {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LlmError {
    #[error("no API key")]
    MissingKey,
    /// The provider rejected the key (401/403).
    #[error("the API key was rejected: {0}")]
    Unauthorized(String),
    #[error("rate limited: {0}")]
    RateLimited(String),
    /// The account is out of credits or over its spend limit.
    #[error("out of credits: {0}")]
    Quota(String),
    /// No connection to the provider (offline, DNS, refused).
    #[error("can't reach the provider: {0}")]
    Offline(String),
    #[error("timed out")]
    Timeout,
    /// The provider cannot serve the request now: overloaded, Ollama not
    /// running, model missing.
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("failed: {0}")]
    Failed(String),
    #[error("cancelled")]
    Cancelled,
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
