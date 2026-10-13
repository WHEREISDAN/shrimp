//! The AI edit pipeline between STT and insertion: a deterministic pre-pass,
//! the LLM pass, a deterministic post-pass, and the raw fallback. Whatever
//! fails, the dictation is still inserted (FMT-10, ENG-07).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::guard::{self, Rejection};
use super::library::{Library, Snapshot};
use super::markdown::{self, Rendered, SNIPPET_END, SNIPPET_START};
use super::prompts::{self, PromptOptions};
use super::style::{self, Style, Target};
use super::transforms::AutoApply;
use super::{CleanupLevel, context_format, dictionary, names, snippets, spoken};
use crate::context::DictationContext;
use crate::llm::{LlmError, LlmProviderId, LlmRequest, LlmSource, http};
use crate::notify::{Notice, NoticeAction};
use crate::settings::{Settings, SettingsStore};

/// Ends every notice about a failed AI edit.
pub const RAW_INSERTED: &str = "Inserted the transcript without AI edits.";

/// Names and identifiers from the screen that may reach the speech engine
/// and the AI edit, when context awareness is on (CTX-02, CTX-04).
const MAX_CONTEXT_NAMES: usize = 20;
const MAX_CONTEXT_IDENTIFIERS: usize = 40;

/// Phrases that cancel a whole dictation, so an empty edit is intended.
const CANCEL_PHRASES: [&str; 5] = [
    "never mind",
    "nevermind",
    "scratch that",
    "cancel that",
    "forget it",
];

/// How long the model may take: a base plus a little per word, because
/// output time grows with the dictation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditTimeout {
    pub base: Duration,
    pub per_word: Duration,
    pub max: Duration,
}

impl Default for EditTimeout {
    fn default() -> Self {
        Self {
            base: Duration::from_secs(5),
            per_word: Duration::from_millis(20),
            max: Duration::from_secs(45),
        }
    }
}

impl EditTimeout {
    fn for_text(&self, text: &str) -> Duration {
        let words = text.split_whitespace().count() as u32;
        (self.base + self.per_word * words).min(self.max)
    }
}

/// The text to insert and how it was made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edited {
    pub text: Rendered,
    /// False when the AI edit was off, skipped or failed (raw fallback).
    pub formatted: bool,
    pub notice: Option<Notice>,
    pub llm_ms: Option<u64>,
    /// The dictation ended with "press enter" (CORE-16).
    pub press_enter: bool,
}

/// The words the cleanup prompt lists: dictionary spellings, snippet
/// triggers and terms on screen.
#[derive(Clone, Copy)]
struct PromptTerms<'a> {
    terms: &'a [String],
    triggers: &'a [String],
    screen: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Failure {
    Llm(LlmProviderId, LlmError),
    Rejected(Rejection),
}

pub struct Editor {
    settings: Arc<SettingsStore>,
    llm: Arc<dyn LlmSource>,
    library: Arc<dyn Library>,
    timeout: EditTimeout,
    /// Providers whose missing-key notice was shown this session.
    missing_key_shown: Mutex<HashSet<LlmProviderId>>,
    /// When a transform last ran after a dictation (TRF-05).
    pub(super) last_auto_apply: Mutex<Option<Instant>>,
}

impl Editor {
    pub fn new(
        settings: Arc<SettingsStore>,
        llm: Arc<dyn LlmSource>,
        library: Arc<dyn Library>,
        timeout: EditTimeout,
    ) -> Self {
        Self {
            settings,
            llm,
            library,
            timeout,
            missing_key_shown: Mutex::new(HashSet::new()),
            last_auto_apply: Mutex::new(None),
        }
    }

    pub(super) fn settings(&self) -> Settings {
        self.settings.get()
    }

    pub(super) fn llm(&self) -> &Arc<dyn LlmSource> {
        &self.llm
    }

    /// Dictionary terms for the speech engine, pinned and most used first,
    /// then the names and identifiers on screen when context awareness is on.
    pub async fn vocabulary(&self, context: &DictationContext) -> Vec<String> {
        let mut terms = dictionary::vocabulary(&self.snapshot().await.dictionary);
        for term in self.context_terms(context) {
            if !terms.iter().any(|known| known.eq_ignore_ascii_case(&term)) {
                terms.push(term);
            }
        }
        terms
    }

    /// Terms from the screen that providers may see (CTX-04: none when off).
    fn context_terms(&self, context: &DictationContext) -> Vec<String> {
        if !self.settings.get().privacy.context_awareness {
            return Vec::new();
        }
        let names = context.names().into_iter().take(MAX_CONTEXT_NAMES);
        let identifiers = context
            .identifiers()
            .into_iter()
            .take(MAX_CONTEXT_IDENTIFIERS);
        names.chain(identifiers).collect()
    }

    /// The dictionary and snippets, read on the blocking pool (SQLite).
    async fn snapshot(&self) -> Snapshot {
        let library = Arc::clone(&self.library);
        tokio::task::spawn_blocking(move || library.snapshot())
            .await
            .unwrap_or_default()
    }

    /// Readies the provider while the user is still speaking.
    pub async fn warm_up(&self) {
        let settings = self.settings.get();
        if settings.ai.cleanup == CleanupLevel::None {
            return;
        }
        if let Ok(provider) = self.llm.provider(&settings.ai).await {
            provider.warm_up().await;
        }
    }

    pub async fn edit(&self, raw: &str, context: &DictationContext) -> Edited {
        let settings = self.settings.get();
        let suffix = settings
            .experimental
            .press_enter_command
            .then(|| spoken::press_enter_suffix(raw))
            .flatten();
        let press_enter = suffix.is_some();
        let raw = suffix.as_deref().unwrap_or(raw);
        let mut edited = self.edit_text(raw, context, &settings).await;
        edited.press_enter = press_enter;
        edited
    }

    async fn edit_text(
        &self,
        raw: &str,
        context: &DictationContext,
        settings: &Settings,
    ) -> Edited {
        if raw.trim().is_empty() {
            return Edited {
                text: Rendered::default(),
                formatted: true,
                notice: None,
                llm_ms: None,
                press_enter: false,
            };
        }
        let settings = settings.clone();
        let library = self.snapshot().await;
        let target = context.target();
        if let Some(snippet) = snippets::match_alone(raw, &library.snippets) {
            return Edited {
                text: markdown::render(&format!(
                    "{SNIPPET_START}{}{SNIPPET_END}",
                    snippet.expansion
                )),
                formatted: true,
                notice: None,
                llm_ms: None,
                press_enter: false,
            };
        }

        let terms = dictionary::vocabulary(&library.dictionary);
        let visible_names = context.names();
        let prepared = spoken::line_breaks(raw);
        let (prepared, mut used) = dictionary::correct(&prepared, &library.dictionary);
        let prepared = names::respell(&prepared, &visible_names, &terms);
        let held = snippets::find_triggers(&prepared, &library.snippets);
        let triggers: Vec<String> = held
            .iter()
            .map(|&index| library.snippets[index].trigger.clone())
            .collect();
        let screen_terms = self.context_terms(context);
        let level = settings.ai.cleanup;

        let started = Instant::now();
        let edited = match level {
            CleanupLevel::None => Err(None),
            _ => self
                .llm_pass(
                    &settings,
                    &prepared,
                    target,
                    PromptTerms {
                        terms: &terms,
                        triggers: &triggers,
                        screen: &screen_terms,
                    },
                )
                .await
                .map_err(Some),
        };
        let llm_ms = (level != CleanupLevel::None).then(|| started.elapsed().as_millis() as u64);

        let (body, formatted, mut notice) = match edited {
            Ok(body) => (body, true, None),
            Err(failure) => {
                if let Some(failure) = &failure {
                    log::info!("AI edit not used: {failure:?}");
                }
                let intended_empty = matches!(failure, Some(Failure::Rejected(Rejection::Empty)))
                    && cancels_everything(raw);
                let body = if intended_empty {
                    String::new()
                } else {
                    spoken::punctuation(&prepared)
                };
                let notice = failure.and_then(|failure| self.notice(failure));
                (body, intended_empty, notice)
            }
        };

        let body = names::respell(&body, &visible_names, &terms);
        let styled = self.post_pass(&body, &settings, target, level, &terms, &library);
        let (expanded, kept_placeholders) =
            match snippets::expand(&styled, &held, &library.snippets) {
                Some(expanded) => (expanded, true),
                None => {
                    log::info!("AI edit reworded a snippet trigger; using the unedited text");
                    log::debug!("edit without its trigger: {styled:?}");
                    let fallback = spoken::punctuation(&prepared);
                    let fallback =
                        self.post_pass(&fallback, &settings, target, level, &terms, &library);
                    let expanded = snippets::expand(&fallback, &held, &library.snippets);
                    (expanded.unwrap_or(fallback), false)
                }
            };
        let skip = notice.is_some() || !held.is_empty();
        let expanded = match self.auto_apply(&expanded, skip).await {
            AutoApply::Applied(text) => text,
            AutoApply::Skipped => expanded,
            AutoApply::Failed(failure) => {
                notice = Some(failure);
                expanded
            }
        };
        let mut text = markdown::render(&expanded);
        if text.html.is_none()
            && let Some(around) = context.surroundings()
        {
            let protected: Vec<String> = terms.iter().cloned().chain(visible_names).collect();
            text.plain = context_format::fit(&text.plain, around, &protected);
        }
        used.extend(dictionary::used_terms(&text.plain, &library.dictionary));
        used.sort_unstable();
        used.dedup();
        if !used.is_empty() {
            let library = Arc::clone(&self.library);
            tokio::task::spawn_blocking(move || library.record_uses(&used));
        }
        Edited {
            text,
            formatted: formatted && kept_placeholders,
            notice,
            llm_ms,
            press_enter: false,
        }
    }

    async fn llm_pass(
        &self,
        settings: &Settings,
        text: &str,
        target: Target,
        terms: PromptTerms<'_>,
    ) -> Result<String, Failure> {
        let provider_id = settings.ai.provider;
        let provider = self
            .llm
            .provider(&settings.ai)
            .await
            .map_err(|err| Failure::Llm(provider_id, err))?;
        let options = PromptOptions {
            level: settings.ai.cleanup,
            target,
            style: settings.styles.for_category(target.category),
            terms: terms.terms,
            triggers: terms.triggers,
            screen_terms: terms.screen,
        };
        let request = LlmRequest::single(
            prompts::system_prompt(&options),
            prompts::user_message(text),
            http::output_budget(text),
        );
        let started = Instant::now();
        let response =
            tokio::time::timeout(self.timeout.for_text(text), provider.complete(&request))
                .await
                .map_err(|_| Failure::Llm(provider_id, LlmError::Timeout))?
                .map_err(|err| Failure::Llm(provider_id, err))?;
        log::info!(
            "AI edit by {} {} in {} ms",
            provider_id.label(),
            provider.model(),
            started.elapsed().as_millis()
        );
        let reviewed = guard::review(text, &response.text, settings.ai.cleanup);
        if let Err(rejection) = &reviewed {
            log::debug!("rejected ({rejection:?}): {:?}", response.text);
        }
        reviewed
            .map(|edited| restore_breaks(text, &edited))
            .map_err(Failure::Rejected)
    }

    /// Dictionary spellings again (the model may undo them), then the
    /// style's casing and trailing-period rules.
    fn post_pass(
        &self,
        body: &str,
        settings: &Settings,
        target: Target,
        level: CleanupLevel,
        terms: &[String],
        library: &Snapshot,
    ) -> String {
        let (body, _) = dictionary::correct(body, &library.dictionary);
        if target.ai_app || level == CleanupLevel::None {
            return body;
        }
        let style = settings.styles.for_category(target.category);
        let body = style::trailing_periods(&body, style, target.category);
        if style == Style::VeryCasual {
            style::very_casual(&body, terms)
        } else {
            body
        }
    }

    fn notice(&self, failure: Failure) -> Option<Notice> {
        let Failure::Llm(provider, error) = failure else {
            return None;
        };
        let name = provider.label();
        let raw = RAW_INSERTED;
        let notice = match error {
            LlmError::MissingKey => {
                let first = self
                    .missing_key_shown
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

/// Puts back the dictated line and paragraph breaks when the model kept the
/// lines but merged the separators (a paragraph turned into a line break).
fn restore_breaks(prepared: &str, edited: &str) -> String {
    let wanted = separators(prepared);
    if wanted.iter().all(|separator| *separator == 1) || edited.contains("\n\n") {
        return edited.to_owned();
    }
    let lines: Vec<&str> = edited
        .split('\n')
        .filter(|line| !line.trim().is_empty())
        .collect();
    if lines.len() != wanted.len() + 1 {
        return edited.to_owned();
    }
    let mut out = String::from(lines[0]);
    for (line, breaks) in lines[1..].iter().zip(&wanted) {
        out.push_str(&"\n".repeat(*breaks));
        out.push_str(line);
    }
    out
}

/// How many line breaks separate each pair of non-empty lines.
fn separators(text: &str) -> Vec<usize> {
    let mut separators = Vec::new();
    let mut breaks = 0;
    let mut seen_text = false;
    for line in text.split('\n') {
        if line.trim().is_empty() {
            breaks += 1;
            continue;
        }
        if seen_text {
            separators.push(breaks + 1);
        }
        seen_text = true;
        breaks = 0;
    }
    separators
}

fn cancels_everything(raw: &str) -> bool {
    let lower = raw.to_lowercase();
    CANCEL_PHRASES.iter().any(|phrase| lower.contains(phrase))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_paragraphs_the_model_merged() {
        assert_eq!(
            restore_breaks(
                "first part\n\nsecond part\nthird",
                "First part.\nSecond part.\nThird."
            ),
            "First part.\n\nSecond part.\nThird."
        );
        assert_eq!(restore_breaks("one\ntwo", "One.\nTwo."), "One.\nTwo.");
        assert_eq!(
            restore_breaks("a\n\nb", "A list:\n1. b\n2. c"),
            "A list:\n1. b\n2. c",
            "a different shape is left alone"
        );
    }
}
