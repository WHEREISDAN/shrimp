//! LLM providers for AI edits (cleanup and styles now, Command Mode later)
//! behind one trait: Anthropic, OpenAI, Groq and local Ollama, using the
//! user's own API key from the OS keychain.

mod anthropic;
pub mod catalog;
mod groq;
pub mod http;
pub mod ollama;
mod openai;
mod provider;
mod registry;

pub use anthropic::Anthropic;
pub use groq::Groq;
pub use ollama::Ollama;
pub use openai::OpenAi;
pub use provider::{LlmError, LlmProvider, LlmProviderId, LlmRequest, LlmResponse, Message, Role};
pub use registry::{LlmSource, Providers};
