//! Why an AI edit was not used, and the notice that tells the user (ENG-07).

use std::collections::HashSet;
use std::sync::Mutex;

use super::guard::Rejection;
use super::pipeline::raw_inserted;
use crate::llm::{LlmError, LlmProviderId};
use crate::notify::{Notice, NoticeAction};
use crate::t;

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
        let without_edits = |reason: String| format!("{reason} {}", raw_inserted());
        let notice = match error {
            LlmError::MissingKey => {
                let first = missing_key_shown
                    .lock()
                    .unwrap_or_else(|err| err.into_inner())
                    .insert(provider);
                if !first {
                    return None;
                }
                Notice::warning(t!("notice.ai.missingKey", provider = name))
                    .with_action(NoticeAction::OpenAiSettings)
            }
            LlmError::Unauthorized(_) => {
                Notice::error(without_edits(t!("notice.ai.unauthorized", provider = name)))
                    .with_action(NoticeAction::OpenAiSettings)
            }
            LlmError::Quota(_) => {
                Notice::error(without_edits(t!("notice.ai.quota", provider = name)))
            }
            LlmError::RateLimited(_) => {
                Notice::warning(without_edits(t!("notice.ai.rateLimited", provider = name)))
            }
            LlmError::Offline(_) => {
                Notice::warning(without_edits(t!("notice.ai.offline", provider = name)))
            }
            LlmError::Timeout => {
                Notice::warning(without_edits(t!("notice.ai.timeout", provider = name)))
            }
            LlmError::Unavailable(message) if provider == LlmProviderId::Ollama => {
                let message = if message.contains("Ollama") {
                    capitalized(&message)
                } else {
                    format!("Ollama: {message}")
                };
                Notice::warning(without_edits(format!("{message}.")))
            }
            LlmError::Unavailable(_) => {
                Notice::warning(without_edits(t!("notice.ai.unavailable", provider = name)))
            }
            LlmError::Failed(message) => {
                Notice::error(without_edits(t!("notice.ai.failed", error = message)))
            }
            LlmError::Cancelled => return None,
        };
        Some(notice)
    }
}

/// "no Ollama model is installed" as the start of a notice.
fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}
