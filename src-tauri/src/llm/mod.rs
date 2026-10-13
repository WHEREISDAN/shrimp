//! LLM providers for AI edits (cleanup, styles, Command Mode) behind one
//! trait. Planned providers: Anthropic, OpenAI, Groq and local Ollama, using
//! the user's own API key from the OS keychain.

mod provider;

pub use provider::{LlmError, LlmProvider, LlmProviderId, LlmRequest, LlmResponse, Message, Role};
