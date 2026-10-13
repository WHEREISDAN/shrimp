//! Transforms from their shortcuts (TRF-01..04): start one on the focused
//! text, then show, copy, retry or undo its changes.

use std::sync::Arc;
use std::time::Instant;

use super::Controller;
use crate::context;
use crate::dictation::state::{ControllerInput, DictationState, TransformAction};
use crate::dictation::transform::{LAST_DICTATION_WINDOW, TransformRun, TransformRunner};
use crate::edit::transforms::Transform;
use crate::hotkeys::Action;
use crate::notify::{Notice, NoticeAction};
use crate::platform::InsertText;
use crate::t;

impl Controller {
    fn transform_runner(&self) -> TransformRunner {
        TransformRunner {
            pipeline: self.pipeline.clone(),
            focus: Arc::clone(&self.focus),
        }
    }

    /// Transforms run like a dictation's processing: nothing else starts meanwhile.
    fn begin_work(&mut self) -> bool {
        if self.gestures.is_recording() {
            return false;
        }
        if self.gestures.is_busy() {
            self.ui.notice(Notice::warning(t!("notice.dictation.busy")));
            return false;
        }
        self.gestures.begin_busy();
        self.set_state(DictationState::Processing);
        true
    }

    pub(super) fn start_transform(&mut self, action: Action) {
        let settings = self.settings.get().transforms;
        let transform = match action {
            Action::Polish => Some(Transform::Polish(settings.polish.clone())),
            Action::PromptEngineer => Some(Transform::PromptEngineer(
                settings.prompt_engineer.prompt.clone(),
            )),
            Action::CustomTransform(index) => settings
                .custom
                .get(usize::from(index))
                .filter(|transform| transform.is_complete())
                .cloned()
                .map(Transform::Custom),
            _ => None,
        };
        let Some(transform) = transform else {
            return;
        };
        if !self.begin_work() {
            return;
        }
        let app = self.focus.frontmost_app();
        let pending = context::capture_for_command(Arc::clone(&self.focus), app);
        let last = self.last_dictation();
        let runner = self.transform_runner();
        let tx = self.tx.clone();
        tauri::async_runtime::spawn(async move {
            let outcome = runner.run(transform, pending, last).await;
            let _ = tx.send(ControllerInput::Processed(Box::new(outcome.into())));
        });
    }

    /// The last dictation's text, if it was pasted within the past minute.
    fn last_dictation(&self) -> Option<(String, Instant)> {
        let at = self
            .last_inserted_at
            .filter(|at| at.elapsed() <= LAST_DICTATION_WINDOW)?;
        let text = self.last.as_ref()?.text.as_ref()?.text.plain.clone();
        Some((text, at))
    }

    pub(super) fn transform_action(&mut self, action: TransformAction) {
        let Some(run) = self.last_transform.clone() else {
            if action == TransformAction::View {
                self.ui
                    .notice(Notice::info(t!("notice.transform.nothingToShow")));
            }
            return;
        };
        match action {
            TransformAction::View => self.ui.changes(run.view()),
            TransformAction::Copy => {
                let inserter = Arc::clone(&self.inserter);
                let ui = Arc::clone(&self.ui);
                let text = InsertText::plain(run.result);
                tauri::async_runtime::spawn_blocking(move || match inserter.copy(&text) {
                    Ok(()) => ui.notice(Notice::info(t!("notice.copied"))),
                    Err(error) => ui.notice(Notice::error(t!("notice.copyFailed", error = error))),
                });
            }
            TransformAction::Retry | TransformAction::Undo => {
                if !self.begin_work() {
                    return;
                }
                self.retrying = action == TransformAction::Retry;
                let runner = self.transform_runner();
                let tx = self.tx.clone();
                tauri::async_runtime::spawn(async move {
                    let outcome = if action == TransformAction::Retry {
                        runner.retry(&run).await
                    } else {
                        runner.undo(&run).await
                    };
                    let _ = tx.send(ControllerInput::Processed(Box::new(outcome.into())));
                });
            }
        }
    }

    /// A transform (or a retry) replaced the text.
    pub(super) fn transformed(&mut self, run: &TransformRun) {
        if std::mem::take(&mut self.retrying) {
            self.ui.changes(run.view());
        } else if run.result.trim() == run.original.trim() {
            self.ui
                .notice(Notice::info(t!("notice.transform.noChanges")));
        } else {
            self.ui.notice(
                Notice::info(t!("notice.transform.done"))
                    .with_action(NoticeAction::ViewTransformChanges),
            );
        }
        self.last_transform = Some(run.clone());
    }
}
