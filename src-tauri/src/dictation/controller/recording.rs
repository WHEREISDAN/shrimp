//! Recording new shortcuts for the Shortcuts dialog (KEYS-01..07), and the
//! shortcut events that are not gestures: another app's shortcut during a
//! held dictation (CORE-12) and Secure Keyboard Entry (KEYS-09).

use std::time::Duration;

use super::Controller;
use crate::dictation::bindings;
use crate::dictation::gestures::{CancelReason, RecordingMode};
use crate::dictation::state::RecordingStatus;
use crate::hotkeys::{Action, RecordUpdate, Recording};
use crate::notify::Notice;
use crate::t;

/// Another key this soon after a held dictation starts was meant for
/// another app (CORE-12).
const INTERRUPT_WINDOW: Duration = Duration::from_secs(1);

impl Controller {
    /// The first binding of `action`, formatted for tips.
    pub(super) fn label_of(&self, action: Action) -> String {
        bindings::triggers(action, &self.bindings, &self.settings.get())
            .first()
            .map(self.label)
            .unwrap_or_else(|| action.label())
    }

    pub(super) fn record_shortcut(&mut self, action: Action, index: Option<usize>) {
        if self.recording.is_some() || self.gestures.is_busy() {
            self.ui.shortcut_recording(RecordingStatus::Failed {
                action,
                message: t!("notice.shortcuts.finishDictating"),
            });
            return;
        }
        if let Some((previous, _)) = self.shortcut_target.take() {
            self.ui
                .shortcut_recording(RecordingStatus::Cancelled { action: previous });
        }
        self.shortcut_target = Some((action, index));
        self.hotkeys
            .set_recording(Some(Recording::new(action == Action::Cancel)));
        self.ui.shortcut_recording(RecordingStatus::Recording {
            action,
            preview: None,
        });
    }

    pub(super) fn cancel_recording(&mut self) {
        if let Some((action, _)) = self.shortcut_target {
            self.end_recording();
            self.ui
                .shortcut_recording(RecordingStatus::Cancelled { action });
        }
    }

    pub(super) fn recording_update(&mut self, update: RecordUpdate) {
        let Some((action, index)) = self.shortcut_target else {
            return;
        };
        let status = match update {
            RecordUpdate::Preview(trigger) => {
                self.ui.shortcut_recording(RecordingStatus::Recording {
                    action,
                    preview: Some((self.label)(&trigger)),
                });
                return;
            }
            RecordUpdate::Cancelled => RecordingStatus::Cancelled { action },
            RecordUpdate::Done(recorded) if recorded.both_sides => RecordingStatus::Failed {
                action,
                message: t!("error.hotkeys.bothSides"),
            },
            RecordUpdate::Done(recorded) => {
                let settings = self.settings.get();
                let saved = bindings::with_trigger(
                    &self.bindings,
                    &settings,
                    action,
                    index,
                    recorded.trigger,
                    self.label,
                    self.reserved,
                )
                .and_then(|(patch, warning)| {
                    self.settings
                        .update(&patch)
                        .map(|_| warning)
                        .map_err(|error| error.to_string())
                });
                match saved {
                    Ok(warning) => RecordingStatus::Saved {
                        action,
                        shortcut: (self.label)(&recorded.trigger),
                        warning,
                    },
                    Err(message) => RecordingStatus::Failed { action, message },
                }
            }
        };
        self.end_recording();
        self.ui.shortcut_recording(status);
    }

    fn end_recording(&mut self) {
        self.shortcut_target = None;
        self.hotkeys.set_recording(None);
        self.refresh_bindings();
    }

    /// A key that is no shortcut, pressed early in a held dictation, was
    /// another app's shortcut: the dictation is cancelled (CORE-12).
    pub(super) fn interrupted(&mut self) {
        let early = self
            .recording
            .as_ref()
            .is_some_and(|recording| recording.started.elapsed() < INTERRUPT_WINDOW);
        if early && self.gestures.mode() == Some(RecordingMode::Hold) {
            log::info!("another shortcut was pressed; cancelling the dictation");
            self.gestures.reset();
            self.cancel(CancelReason::Interrupted);
        }
    }

    /// Explains once each time Secure Keyboard Entry turns on. macOS names
    /// the frontmost app when a background process turned it on, so the name
    /// is only a hint.
    pub(super) fn secure_input_changed(&mut self, owner: Option<String>) {
        let was_on =
            std::mem::replace(&mut *super::lock(&self.secure_input), owner.clone()).is_some();
        if let (Some(app), false) = (owner, was_on) {
            let message = t!("notice.shortcuts.secureInput", app = app);
            self.ui.notice(Notice::warning(message));
        }
    }
}
