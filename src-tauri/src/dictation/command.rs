//! A Command Mode session after recording (CMD-01..05): transcribe the
//! instruction, find the text it applies to, then open a search, replace the
//! text or show an answer. The user's text is only replaced when it is still
//! selected as it was; otherwise the result stays on the clipboard.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use super::pipeline::Pipeline;
use super::state::{Answer, Delivered, Outcome, Processed};
use crate::audio::AudioClip;
use crate::context::{AppKind, DictationContext, PendingContext};
use crate::edit::command::{CommandInput, CommandReply, MAX_COMMAND_WORDS, drops_text};
use crate::notify::Notice;
use crate::platform::{
    ContextRequest, CopyReason, Editable, FieldContext, FieldKind, FocusInspector, InsertOptions,
    InsertText, Launcher, SyntheticKey, TextInserter, TextRange,
};

/// How long Command Mode waits for the selection to be read.
const CONTEXT_WAIT: Duration = Duration::from_secs(1);
/// Where "search …" instructions go, as in Wispr Flow.
const SEARCH_URL: &str = "https://www.perplexity.ai/search?q=";
const SEARCH_WORDS: [&str; 5] = [
    "search for ",
    "search ",
    "look up ",
    "google ",
    "find online ",
];

static NEXT_ANSWER: AtomicU64 = AtomicU64::new(1);

/// What the instruction applies to, and how a result replaces it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    /// Selected text read through Accessibility, replaced only if unchanged.
    Selection { field_id: u64, text: String },
    /// Selected text copied with Cmd+C because Accessibility couldn't read
    /// it; replaced only where the focus may take text.
    CopiedSelection { text: String, editable: bool },
    /// The field's whole text, selected again before pasting.
    Field {
        field_id: u64,
        text: String,
        length: usize,
    },
    /// An empty text field: results are pasted.
    EmptyField,
    /// No text field: results are shown (CMD-03).
    NoField,
}

impl Target {
    fn input(&self) -> CommandInput<'_> {
        match self {
            Self::Selection { text, .. } | Self::CopiedSelection { text, .. } => {
                CommandInput::Selection(text)
            }
            Self::Field { text, .. } => CommandInput::Document(text),
            Self::EmptyField | Self::NoField => CommandInput::Nothing,
        }
    }
}

pub struct CommandRunner {
    pub pipeline: Pipeline,
    pub focus: Arc<dyn FocusInspector>,
    pub launcher: Arc<dyn Launcher>,
}

impl CommandRunner {
    pub async fn run(&self, clip: AudioClip, mut context: PendingContext) -> Processed {
        let (instruction, _language, mut notices) =
            match self.pipeline.transcribe_clip(clip, &context.ready()).await {
                Ok(transcribed) => transcribed,
                Err(outcome) => return outcome.into(),
            };
        let instruction = instruction.trim().to_owned();
        let context = context.settled(CONTEXT_WAIT).await;
        let target = self.target(&context).await;

        if let Some(query) = search_query(&instruction) {
            return self.search(&query, &target).into();
        }
        if let CommandInput::Selection(text) | CommandInput::Document(text) = target.input()
            && text.split_whitespace().count() > MAX_COMMAND_WORDS
        {
            return Outcome::CommandFailed(format!(
                "That text is too long for Command Mode (over {MAX_COMMAND_WORDS} words). Select less text."
            ))
            .into();
        }
        if target == Target::NoField && context.field.is_some() && context_field_too_long(&context)
        {
            return Outcome::CommandFailed(
                "That field is too long for Command Mode. Select the text to change.".into(),
            )
            .into();
        }

        let reply = match self
            .pipeline
            .editor
            .command(&instruction, target.input())
            .await
        {
            Ok((reply, _)) => reply,
            Err(error) => {
                log::info!("command failed: {error}");
                return Outcome::CommandFailed(format!(
                    "Command Mode didn't get a result ({error}). Nothing was changed."
                ))
                .into();
            }
        };
        let reply = match (reply, target.input()) {
            (
                CommandReply::Edit(text),
                CommandInput::Selection(original) | CommandInput::Document(original),
            ) if drops_text(&instruction, original, &text) => {
                log::info!("command result left out most of the text; shown instead");
                notices.push(Notice::warning(
                    "The result left out most of your text, so it wasn't pasted.",
                ));
                CommandReply::Answer(text)
            }
            (reply, _) => reply,
        };
        let outcome = match (reply, &target) {
            (CommandReply::Edit(text) | CommandReply::Answer(text), Target::EmptyField) => {
                self.paste(text, &context, None).await
            }
            (
                CommandReply::Edit(text),
                Target::Selection {
                    field_id,
                    text: selected,
                },
            ) => {
                self.replace_selection(text, &context, *field_id, selected)
                    .await
            }
            (CommandReply::Edit(text), Target::CopiedSelection { editable: true, .. }) => {
                self.paste(text, &context, None).await
            }
            (
                CommandReply::Edit(text),
                Target::Field {
                    field_id, length, ..
                },
            ) => {
                let whole = TextRange {
                    start: 0,
                    len: *length,
                };
                self.paste(text, &context, Some((*field_id, whole))).await
            }
            (CommandReply::Edit(text) | CommandReply::Answer(text), _) => {
                Outcome::Answered(Answer {
                    id: NEXT_ANSWER.fetch_add(1, Ordering::Relaxed),
                    question: instruction,
                    text,
                })
            }
        };
        if matches!(outcome, Outcome::Copied(_, CopyReason::TargetChanged)) {
            notices.push(Notice::warning(
                "The selection changed, so the result is on the clipboard.",
            ));
        }
        Processed {
            language_check: None,
            outcome,
            notices,
            learn: None,
        }
    }

    /// The selection (read, or copied as a fallback), the field's text, or nothing.
    async fn target(&self, context: &DictationContext) -> Target {
        let field = context.readable_field();
        if let Some(field) = field {
            if field.selection.is_some_and(|range| range.len > 0)
                && !field.selected_text.trim().is_empty()
            {
                return Target::Selection {
                    field_id: field.field_id,
                    text: field.selected_text.clone(),
                };
            }
            if field.selection.is_some_and(|range| range.len == 0) {
                return match &field.text {
                    Some(text) if !text.trim().is_empty() => Target::Field {
                        field_id: field.field_id,
                        text: text.clone(),
                        length: field.length,
                    },
                    Some(_) => Target::EmptyField,
                    None if field.length == 0 => Target::EmptyField,
                    // Too long to read whole; only a selection can be changed.
                    None => Target::NoField,
                };
            }
        }
        let secure = context.field_kind() == FieldKind::Secure;
        if secure || context.app_kind() == AppKind::RemoteDesktop {
            return Target::NoField;
        }
        // Accessibility couldn't say what is selected: copy it (CMD-02).
        let inserter = Arc::clone(&self.pipeline.inserter);
        let copied = tokio::task::spawn_blocking(move || inserter.copy_selection())
            .await
            .ok()
            .and_then(Result::ok)
            .flatten();
        let editable = field.is_some_and(|field| field.editable != Editable::No);
        match copied {
            Some(text) if !text.trim().is_empty() => Target::CopiedSelection { text, editable },
            _ if editable => Target::EmptyField,
            _ => Target::NoField,
        }
    }

    fn search(&self, query: &str, target: &Target) -> Outcome {
        let mut query = query.to_owned();
        if let Target::Selection { text, .. } | Target::CopiedSelection { text, .. } = target {
            query.push(' ');
            query.push_str(text.trim());
        }
        let url = format!("{SEARCH_URL}{}", encode_query(&query));
        match self.launcher.open_url(&url) {
            Ok(()) => Outcome::Searched,
            Err(error) => Outcome::CommandFailed(format!("Couldn't open the search: {error}")),
        }
    }

    /// Pastes over the selection if it is still the text the command read.
    async fn replace_selection(
        &self,
        text: String,
        context: &DictationContext,
        field_id: u64,
        selected: &str,
    ) -> Outcome {
        let focus = Arc::clone(&self.focus);
        let now = tokio::task::spawn_blocking(move || focus.read_field(&recheck()))
            .await
            .ok()
            .flatten();
        let unchanged = now.as_ref().is_some_and(|field: &FieldContext| {
            field.field_id == field_id && field.selected_text == selected
        });
        if !unchanged || !self.same_app(context) {
            return self.copy_instead(text).await;
        }
        self.paste(text, context, None).await
    }

    /// Pastes `text`, first selecting `range` in the field when given.
    async fn paste(
        &self,
        text: String,
        context: &DictationContext,
        select: Option<(u64, TextRange)>,
    ) -> Outcome {
        if !self.same_app(context) {
            return self.copy_instead(text).await;
        }
        if let Some((field_id, range)) = select {
            let focus = Arc::clone(&self.focus);
            let inserter = Arc::clone(&self.pipeline.inserter);
            let selected = tokio::task::spawn_blocking(move || {
                focus.select_range(field_id, range)
                    || inserter
                        .press(SyntheticKey::SelectAll, handy_keys::Modifiers::empty())
                        .is_ok()
            })
            .await
            .unwrap_or(false);
            if !selected {
                return self.copy_instead(text).await;
            }
        }
        let delivered = Delivered {
            text: InsertText::plain(text),
            formatted: true,
        };
        let options: InsertOptions = self.pipeline.insert_options(context);
        self.pipeline
            .insert(delivered, options, Duration::ZERO)
            .await
    }

    fn same_app(&self, context: &DictationContext) -> bool {
        let now = self.focus.frontmost_app();
        match (&context.app, &now) {
            (Some(then), Some(now)) => then.bundle_id == now.bundle_id,
            _ => true,
        }
    }

    async fn copy_instead(&self, text: String) -> Outcome {
        let delivered = Delivered {
            text: InsertText::plain(text),
            formatted: true,
        };
        let inserter: Arc<dyn TextInserter> = Arc::clone(&self.pipeline.inserter);
        let copy = delivered.text.clone();
        match tokio::task::spawn_blocking(move || inserter.copy(&copy)).await {
            Ok(Ok(())) => Outcome::Copied(delivered, CopyReason::TargetChanged),
            Ok(Err(error)) => Outcome::InsertFailed(delivered, error.to_string()),
            Err(error) => Outcome::InsertFailed(delivered, error.to_string()),
        }
    }
}

/// A quick read of the selection, to confirm it before replacing it.
fn recheck() -> ContextRequest {
    ContextRequest::default()
}

fn context_field_too_long(context: &DictationContext) -> bool {
    context
        .readable_field()
        .is_some_and(|field| field.text.is_none() && field.length > 0)
}

/// "Search for X" and similar give the query X (CMD-05).
fn search_query(instruction: &str) -> Option<String> {
    let lower = instruction.to_lowercase();
    let lower = lower.trim_start_matches(|c: char| !c.is_alphanumeric());
    SEARCH_WORDS.iter().find_map(|word| {
        let rest = lower.strip_prefix(word)?;
        let start = instruction.len() - rest.len();
        let query = instruction[start..]
            .trim()
            .trim_end_matches(['.', '?', '!'])
            .trim();
        Some(query.to_owned())
    })
}

/// Percent-encodes a search query for a URL.
fn encode_query(query: &str) -> String {
    query
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            b' ' => "+".to_owned(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_instructions_give_their_query() {
        assert_eq!(
            search_query("Search for flights to Lisbon."),
            Some("flights to Lisbon".into())
        );
        assert_eq!(search_query("search this"), Some("this".into()));
        assert_eq!(
            search_query("Look up Rust traits"),
            Some("Rust traits".into())
        );
        assert_eq!(search_query("make this a search box"), None);
        assert_eq!(search_query("searching is hard"), None);
    }

    #[test]
    fn queries_are_url_encoded() {
        assert_eq!(encode_query("what is a café?"), "what+is+a+caf%C3%A9%3F");
    }
}
