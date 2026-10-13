//! What a finished session tells the user, and what paste and copy last
//! transcript remember from it.

use super::state::{Delivered, DictationUi, LastDictation, LastStatus, Outcome};
use crate::notify::{Notice, NoticeAction};
use crate::platform::{CopyReason, InsertText};

/// Shows the notices or the answer for `outcome`. Returns what to remember
/// for paste and copy last transcript, or None to keep the previous one.
pub fn report(outcome: Outcome, ui: &dyn DictationUi) -> Option<LastDictation> {
    let remember = |status, text, clip| Some(LastDictation { status, text, clip });
    match outcome {
        Outcome::Inserted(text) => remember(LastStatus::Inserted, Some(text), None),
        Outcome::Copied(text, reason) => {
            match reason {
                CopyReason::NoTextField => ui.notice(
                    Notice::warning(
                        "No text field is focused. Your transcript is on the clipboard.",
                    )
                    .with_action(NoticeAction::CopyLastTranscript),
                ),
                CopyReason::AccessibilityMissing => ui.notice(
                    Notice::error(
                        "Shrimp needs Accessibility to paste. Your transcript is on the clipboard.",
                    )
                    .with_action(NoticeAction::OpenAccessibilitySettings),
                ),
                // The command explains it in its own notice.
                CopyReason::TargetChanged => {}
            }
            remember(LastStatus::Copied, Some(text), None)
        }
        Outcome::NoSpeech => {
            ui.notice(Notice::info("No speech detected"));
            None
        }
        Outcome::NothingToInsert => {
            ui.notice(Notice::info("Nothing to insert"));
            None
        }
        Outcome::RecordingFailed(message) => {
            ui.notice(Notice::error(format!("Recording failed: {message}")));
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
                Notice::error(format!("Couldn't transcribe: {message}"))
            });
            remember(LastStatus::Failed, None, Some(clip))
        }
        Outcome::InsertFailed(text, message) => {
            ui.notice(
                Notice::error(format!("Couldn't paste: {message}"))
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
        Outcome::CommandFailed(message) => {
            ui.notice(Notice::error(message));
            None
        }
    }
}
