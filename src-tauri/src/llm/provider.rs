use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LlmProviderId {
    Anthropic,
    OpenAi,
    Groq,
    Ollama,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmResponse {
    pub text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    /// The provider cannot run yet: API key missing, Ollama not running, offline.
    #[error("AI provider unavailable: {0}")]
    Unavailable(String),
    #[error("AI request failed: {0}")]
    Failed(String),
    #[error("AI request cancelled")]
    Cancelled,
}

/// One LLM backend, configured with its model when constructed.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn id(&self) -> LlmProviderId;

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError>;
}
