//! Running a transform from its shortcut (TRF-01..04, TRF-06): find the text
//! (the selection, else the field, else the last dictation of the past
//! minute in an empty field), rewrite it, and replace it in place. The run
//! is kept so its changes can be shown, retried or undone.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;

use super::pipeline::Pipeline;
use super::state::{Delivered, Outcome};
use crate::context::{DictationContext, PendingContext};
use crate::edit::transforms::{MAX_TRANSFORM_WORDS, Transform};
use crate::platform::{
    AppIdentity, ContextRequest, Editable, FocusInspector, InsertText, SyntheticKey, TextRange,
};
use crate::t;

const CONTEXT_WAIT: Duration = Duration::from_secs(1);
/// The last dictation can be transformed this long after it was pasted.
pub const LAST_DICTATION_WINDOW: Duration = Duration::from_secs(60);

static NEXT_RUN: AtomicU64 = AtomicU64::new(1);

/// Where a transform's result sits, so Undo and Retry can select it again.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Placement {
    /// The result starts at `start` (UTF-16) in the field `field_id`.
    Field { field_id: u64, start: usize },
    /// Pasted where Accessibility can't follow; Undo falls back to Cmd+Z.
    Unknown,
}

/// A finished transform, for the changes view (TRF-04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformRun {
    pub id: u64,
    pub transform: Transform,
    pub original: String,
    pub result: String,
    app: Option<AppIdentity>,
    placement: Placement,
}

/// What the Flow Bar's changes view shows. Mirrors `TransformView` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformView {
    pub id: u64,
    pub name: String,
    pub original: String,
    pub result: String,
}

impl TransformRun {
    pub fn view(&self) -> TransformView {
        TransformView {
            id: self.id,
            name: self.transform.name(),
            original: self.original.clone(),
            result: self.result.clone(),
        }
    }
}

/// The text a transform works on and where its result goes.
enum Source {
    Selection {
        field_id: u64,
        start: usize,
        text: String,
    },
    Field {
        field_id: u64,
        length: usize,
        text: String,
    },
    LastDictation {
        field_id: u64,
        cursor: usize,
        text: String,
    },
    Copied(String),
}

impl Source {
    fn text(&self) -> &str {
        match self {
            Self::Selection { text, .. }
            | Self::Field { text, .. }
            | Self::LastDictation { text, .. }
            | Self::Copied(text) => text,
        }
    }
}

pub struct TransformRunner {
    pub pipeline: Pipeline,
    pub focus: Arc<dyn FocusInspector>,
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn failed(error: impl ToString) -> Outcome {
    Outcome::TransformFailed(error.to_string())
}

impl TransformRunner {
    /// `last` is the last dictation's text and when it was pasted.
    pub async fn run(
        &self,
        transform: Transform,
        mut context: PendingContext,
        last: Option<(String, Instant)>,
    ) -> Outcome {
        let context = context.settled(CONTEXT_WAIT).await;
        let source = match self.source(&context, last).await {
            Ok(source) => source,
            Err(outcome) => return outcome,
        };
        let words = source.text().split_whitespace().count();
        if words == 0 {
            return failed(t!("notice.transform.selectText"));
        }
        if words > MAX_TRANSFORM_WORDS {
            let max = MAX_TRANSFORM_WORDS;
            return failed(match transform {
                Transform::Polish(_) => t!("notice.transform.polishTooLong", max = max),
                _ => t!("notice.transform.tooLong", max = max),
            });
        }
        let result = match self
            .pipeline
            .editor
            .transform(&transform, source.text(), None)
            .await
        {
            Ok((result, _)) => result,
            Err(error) => return failed(error),
        };
        let original = source.text().to_owned();
        let placement = match self.place(&source, &result, &context).await {
            Ok(placement) => placement,
            Err(outcome) => return outcome,
        };
        Outcome::Transformed(Box::new(TransformRun {
            id: NEXT_RUN.fetch_add(1, Ordering::Relaxed),
            transform,
            original,
            result,
            app: context.app.clone(),
            placement,
        }))
    }

    async fn source(
        &self,
        context: &DictationContext,
        last: Option<(String, Instant)>,
    ) -> Result<Source, Outcome> {
        if let Some(field) = context.readable_field() {
            match field.selection {
                Some(range) if range.len > 0 && !field.selected_text.trim().is_empty() => {
                    return Ok(Source::Selection {
                        field_id: field.field_id,
                        start: range.start,
                        text: field.selected_text.clone(),
                    });
                }
                Some(range) => {
                    let text = field.text.clone().unwrap_or_default();
                    if !text.trim().is_empty() {
                        return Ok(Source::Field {
                            field_id: field.field_id,
                            length: field.length,
                            text,
                        });
                    }
                    if field.length > 0 {
                        return Err(failed(t!("notice.transform.selectLess")));
                    }
                    return match last.filter(|(_, at)| at.elapsed() <= LAST_DICTATION_WINDOW) {
                        Some((text, _)) => Ok(Source::LastDictation {
                            field_id: field.field_id,
                            cursor: range.start,
                            text,
                        }),
                        None => Err(failed(t!("notice.transform.selectText"))),
                    };
                }
                None => {}
            }
        }
        let inserter = Arc::clone(&self.pipeline.inserter);
        let copied = tokio::task::spawn_blocking(move || inserter.copy_selection())
            .await
            .ok()
            .and_then(Result::ok)
            .flatten();
        let editable = context
            .readable_field()
            .is_some_and(|field| field.editable != Editable::No);
        match copied {
            Some(text) if editable && !text.trim().is_empty() => Ok(Source::Copied(text)),
            _ if context.field.is_none() => Err(failed(t!("notice.transform.noTextBox"))),
            _ => Err(failed(t!("notice.transform.selectText"))),
        }
    }

    /// Replaces the source with `result` and says where it now sits.
    async fn place(
        &self,
        source: &Source,
        result: &str,
        context: &DictationContext,
    ) -> Result<Placement, Outcome> {
        if !self.same_app(context.app.as_ref()) {
            return Err(self.copy_instead(result).await);
        }
        let placement = match source {
            Source::Selection {
                field_id,
                start,
                text,
            } => {
                let now = self.read().await;
                let unchanged = now
                    .as_ref()
                    .is_some_and(|(id, selected)| id == field_id && selected == text);
                if !unchanged {
                    return Err(self.copy_instead(result).await);
                }
                Placement::Field {
                    field_id: *field_id,
                    start: *start,
                }
            }
            Source::Field {
                field_id, length, ..
            } => {
                let whole = TextRange {
                    start: 0,
                    len: *length,
                };
                if !self.select(*field_id, whole).await {
                    return Err(self.copy_instead(result).await);
                }
                Placement::Field {
                    field_id: *field_id,
                    start: 0,
                }
            }
            Source::LastDictation {
                field_id, cursor, ..
            } => Placement::Field {
                field_id: *field_id,
                start: *cursor,
            },
            Source::Copied(_) => Placement::Unknown,
        };
        match self.paste(result, context).await {
            Outcome::Inserted(_) => Ok(placement),
            other => Err(other),
        }
    }

    /// Puts the original back (TRF-04 Undo).
    pub async fn undo(&self, run: &TransformRun) -> Outcome {
        self.replace_result(run, &run.original).await
    }

    /// Runs the transform on the original again and swaps the new result in.
    pub async fn retry(&self, run: &TransformRun) -> Outcome {
        let result = match self
            .pipeline
            .editor
            .transform(&run.transform, &run.original, Some(&run.result))
            .await
        {
            Ok((result, _)) => result,
            Err(error) => return failed(error),
        };
        match self.replace_result(run, &result).await {
            Outcome::TransformUndone => Outcome::Transformed(Box::new(TransformRun {
                id: NEXT_RUN.fetch_add(1, Ordering::Relaxed),
                result,
                ..run.clone()
            })),
            other => other,
        }
    }

    /// Replaces the run's result in the field with `text`, if the result is
    /// still there; Cmd+Z when it can't be found but the app is the same.
    async fn replace_result(&self, run: &TransformRun, text: &str) -> Outcome {
        let context = DictationContext::new(run.app.clone(), None);
        if !self.same_app(run.app.as_ref()) {
            return self.copy_instead(text).await;
        }
        match run.placement {
            Placement::Field { field_id, start } => {
                let range = TextRange {
                    start,
                    len: utf16_len(&run.result),
                };
                let still_there = self.read_full().await.is_some_and(|(id, full)| {
                    let units: Vec<u16> = full.encode_utf16().collect();
                    id == field_id
                        && units
                            .get(range.start..range.end())
                            .is_some_and(|slice| String::from_utf16_lossy(slice) == run.result)
                });
                if !still_there || !self.select(field_id, range).await {
                    return self.copy_instead(text).await;
                }
                match self.paste(text, &context).await {
                    Outcome::Inserted(_) => Outcome::TransformUndone,
                    other => other,
                }
            }
            Placement::Unknown if text == run.original => {
                let inserter = Arc::clone(&self.pipeline.inserter);
                let undone = tokio::task::spawn_blocking(move || {
                    inserter.press(SyntheticKey::Undo, handy_keys::Modifiers::empty())
                })
                .await;
                match undone {
                    Ok(Ok(())) => Outcome::TransformUndone,
                    _ => self.copy_instead(text).await,
                }
            }
            Placement::Unknown => self.copy_instead(text).await,
        }
    }

    /// The focused field and its selected text, read again.
    async fn read(&self) -> Option<(u64, String)> {
        let focus = Arc::clone(&self.focus);
        tokio::task::spawn_blocking(move || focus.read_field(&ContextRequest::default()))
            .await
            .ok()
            .flatten()
            .map(|field| (field.field_id, field.selected_text))
    }

    async fn read_full(&self) -> Option<(u64, String)> {
        let focus = Arc::clone(&self.focus);
        let request = ContextRequest {
            text_limit: 40_000,
            ..ContextRequest::default()
        };
        tokio::task::spawn_blocking(move || focus.read_field(&request))
            .await
            .ok()
            .flatten()
            .and_then(|field| field.text.map(|text| (field.field_id, text)))
    }

    async fn select(&self, field_id: u64, range: TextRange) -> bool {
        let focus = Arc::clone(&self.focus);
        let inserter = Arc::clone(&self.pipeline.inserter);
        let whole = range.start == 0;
        tokio::task::spawn_blocking(move || {
            focus.select_range(field_id, range)
                || whole
                    && inserter
                        .press(SyntheticKey::SelectAll, handy_keys::Modifiers::empty())
                        .is_ok()
        })
        .await
        .unwrap_or(false)
    }

    async fn paste(&self, text: &str, context: &DictationContext) -> Outcome {
        let delivered = Delivered {
            text: InsertText::plain(text),
            formatted: true,
        };
        let options = self.pipeline.insert_options(context);
        self.pipeline
            .insert(delivered, options, Duration::ZERO)
            .await
    }

    fn same_app(&self, app: Option<&AppIdentity>) -> bool {
        match (app, self.focus.frontmost_app()) {
            (Some(then), Some(now)) => then.bundle_id == now.bundle_id,
            _ => true,
        }
    }

    async fn copy_instead(&self, text: &str) -> Outcome {
        let delivered = Delivered {
            text: InsertText::plain(text),
            formatted: true,
        };
        let inserter = Arc::clone(&self.pipeline.inserter);
        let copy = delivered.text.clone();
        match tokio::task::spawn_blocking(move || inserter.copy(&copy)).await {
            Ok(Ok(())) => Outcome::TransformCopied(delivered),
            Ok(Err(error)) => Outcome::InsertFailed(delivered, error.to_string()),
            Err(error) => Outcome::InsertFailed(delivered, error.to_string()),
        }
    }
}
