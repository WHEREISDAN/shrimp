//! Why an AI edit was not used, and the notice that tells the user (ENG-07).

use std::collections::HashSet;
use std::sync::Mutex;

use super::guard::Rejection;
use super::pipeline::RAW_INSERTED;
use crate::llm::{LlmError, LlmProviderId};
use crate::notify::{Notice, NoticeAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    Llm(LlmProviderId, LlmError),
    Rejected(Rejection),
}

impl Failure {
    /// The notice for a failed AI edit; a missing key is mentioned once per
    /// provider per session (`missing_key_shown`).
    pub fn notice(self, missing_key_shown: &Mutex<HashSet<LlmProviderId>>) -> Option<Notice> {
        let Failure::Llm(provider, error) = self else {
            return None;
        };
        let name = provider.label();
        let raw = RAW_INSERTED;
        let notice = match error {
            LlmError::MissingKey => {
                let first = missing_key_shown
                    .lock()
                    .unwrap_or_else(|err| err.into_inner())
                    .insert(provider);
                if !first {
                    return None;
                }
                Notice::warning(format!(
                    "AI edits need your {name} API key. Add it in Settings, or turn AI edits off."
                ))
                .with_action(NoticeAction::OpenAiSettings)
            }
            LlmError::Unauthorized(_) => {
                Notice::error(format!("{name} rejected your API key. {raw}"))
                    .with_action(NoticeAction::OpenAiSettings)
            }
            LlmError::Quota(_) => {
                Notice::error(format!("Your {name} account is out of credits. {raw}"))
            }
            LlmError::RateLimited(_) => {
                Notice::warning(format!("{name} rate limit reached. {raw}"))
            }
            LlmError::Offline(_) => Notice::warning(format!("Can't reach {name}. {raw}")),
            LlmError::Timeout => Notice::warning(format!("{name} took too long. {raw}")),
            LlmError::Unavailable(message) if provider == LlmProviderId::Ollama => {
                let message = if message.contains("Ollama") {
                    message
                } else {
                    format!("Ollama: {message}")
                };
                Notice::warning(format!("{message}. {raw}"))
            }
            LlmError::Unavailable(_) => Notice::warning(format!("{name} is unavailable. {raw}")),
            LlmError::Failed(message) => {
                Notice::error(format!("AI edit failed ({message}). {raw}"))
            }
            LlmError::Cancelled => return None,
        };
        Some(notice)
    }
}
