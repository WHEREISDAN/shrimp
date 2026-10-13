//! What a finished session tells the user, and what paste and copy last
//! transcript remember from it.

use super::state::{Delivered, DictationUi, LastDictation, LastStatus, Outcome};
use crate::notify::{Notice, NoticeAction};
use crate::platform::{CopyReason, InsertText};
use crate::t;

/// Shows the notices or the answer for `outcome`. Returns what to remember
/// for paste and copy last transcript, or None to keep the previous one.
pub fn report(outcome: Outcome, ui: &dyn DictationUi) -> Option<LastDictation> {
    let remember = |status, text, clip| {
        Some(LastDictation {
            status,
            text,
            clip,
            history_id: None,
        })
    };
    match outcome {
        Outcome::Inserted(text) => remember(LastStatus::Inserted, Some(text), None),
        Outcome::Copied(text, reason) => {
            match reason {
                CopyReason::NoTextField => ui.notice(
                    Notice::warning(t!("notice.paste.noTextField"))
                        .with_action(NoticeAction::CopyLastTranscript),
                ),
                CopyReason::AccessibilityMissing => ui.notice(
                    Notice::error(t!("notice.paste.needAccessibility"))
                        .with_action(NoticeAction::OpenAccessibilitySettings),
                ),
                // The command explains it in its own notice.
                CopyReason::TargetChanged => {}
            }
            remember(LastStatus::Copied, Some(text), None)
        }
        Outcome::NoSpeech => {
            ui.notice(Notice::info(t!("notice.dictation.noSpeech")));
            None
        }
        Outcome::NothingToInsert => {
            ui.notice(Notice::info(t!("notice.dictation.nothingToInsert")));
            None
        }
        Outcome::RecordingFailed(message) => {
            let message = t!("notice.dictation.recordingFailed", error = message);
            ui.notice(Notice::error(message));
            None
        }
        Outcome::SttFailed {
            clip,
            message,
            unavailable,
        } => {
            ui.notice(if unavailable {
                Notice::warning(message)
            } else {
                Notice::error(t!("notice.dictation.transcribeFailed", error = message))
            });
            remember(LastStatus::Failed, None, Some(clip))
        }
        Outcome::InsertFailed(text, message) => {
            ui.notice(
                Notice::error(t!("notice.paste.failed", error = message))
                    .with_action(NoticeAction::CopyLastTranscript),
            );
            remember(LastStatus::Failed, Some(text), None)
        }
        Outcome::EnterPressed | Outcome::Searched => None,
        Outcome::Answered(answer) => {
            let text = Delivered {
                text: InsertText::plain(answer.text.clone()),
                formatted: true,
            };
            ui.answer(answer);
            remember(LastStatus::Inserted, Some(text), None)
        }
        Outcome::CommandFailed(message) | Outcome::TransformFailed(message) => {
            ui.notice(Notice::error(message));
            None
        }
        Outcome::Transformed(run) => {
            let text = Delivered {
                text: InsertText::plain(run.result.clone()),
                formatted: true,
            };
            remember(LastStatus::Inserted, Some(text), None)
        }
        Outcome::TransformUndone => {
            ui.notice(Notice::info(t!("notice.transform.undone")));
            None
        }
        Outcome::Retried => None,
        Outcome::RetryFailed(message) => {
            ui.notice(Notice::error(message));
            None
        }
        Outcome::TransformCopied(text) => {
            ui.notice(Notice::warning(t!("notice.transform.textChanged")));
            remember(LastStatus::Copied, Some(text), None)
        }
    }
}
