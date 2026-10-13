//! Pastes text into the focused field with the clipboard and Cmd+V, then puts
//! the user's clipboard back (INS-01..04).

use std::sync::mpsc;
use std::time::Duration;

use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation,
};

use super::focus::{self, FocusedField};
use super::{keymap, pasteboard, permissions};
use crate::platform::{CopyReason, InsertOutcome, MainThread, PlatformError, TextInserter};

/// Marks key events Shrimp posts itself ("SHRP"), so they can be told apart
/// from the user's own typing.
pub const SYNTHETIC_EVENT_TAG: i64 = 0x5348_5250;

const MAIN_THREAD_TIMEOUT: Duration = Duration::from_secs(2);

pub struct MacInserter {
    main_thread: MainThread,
}

impl MacInserter {
    pub fn new(main_thread: MainThread) -> Self {
        Self { main_thread }
    }

    fn on_main<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Result<T, PlatformError> {
        let (tx, rx) = mpsc::channel();
        (self.main_thread)(Box::new(move || {
            let _ = tx.send(work());
        }));
        rx.recv_timeout(MAIN_THREAD_TIMEOUT)
            .map_err(|_| PlatformError::Failed("the main thread did not respond".into()))
    }

    fn copy_instead(&self, text: &str, reason: CopyReason) -> Result<InsertOutcome, PlatformError> {
        self.copy(text)?;
        Ok(InsertOutcome::Copied(reason))
    }
}

impl TextInserter for MacInserter {
    fn insert(&self, text: &str, hold: Duration) -> Result<InsertOutcome, PlatformError> {
        if !permissions::accessibility_trusted() {
            return self.copy_instead(text, CopyReason::AccessibilityMissing);
        }
        let field = focus::frontmost_pid().map_or(FocusedField::Unknown, focus::focused_field);
        if field == FocusedField::NotEditable {
            return self.copy_instead(text, CopyReason::NoTextField);
        }

        let text = text.to_owned();
        let (snapshot, written) = self.on_main(move || {
            let snapshot = pasteboard::snapshot();
            let written = pasteboard::write_text(&text, true);
            post_command_key(keymap::paste_key_code());
            (snapshot, written)
        })?;

        let main_thread = self.main_thread.clone();
        std::thread::spawn(move || {
            std::thread::sleep(hold);
            main_thread(Box::new(move || {
                // Something copied in the meantime wins over the old clipboard.
                if pasteboard::change_count() == written {
                    pasteboard::restore(&snapshot);
                } else {
                    log::debug!("clipboard changed after paste; not restoring");
                }
            }));
        });
        Ok(InsertOutcome::Pasted)
    }

    fn copy(&self, text: &str) -> Result<(), PlatformError> {
        let text = text.to_owned();
        self.on_main(move || {
            pasteboard::write_text(&text, false);
        })
    }
}

/// Posts key down and up for `key_code` with exactly Command held, so a
/// physically held Option or Fn cannot change the shortcut.
fn post_command_key(key_code: u16) {
    post_key(key_code, CGEventFlags::MaskCommand);
}

/// Posts after the session event taps, where apps receive input. Keyboard
/// hooks (Shrimp's own included) never see these keys, so a posted Cmd+V
/// cannot leave Command looking held to them.
fn post_key(key_code: u16, flags: CGEventFlags) {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState);
    for key_down in [true, false] {
        let event = CGEvent::new_keyboard_event(source.as_deref(), key_code, key_down);
        CGEvent::set_flags(event.as_deref(), flags);
        CGEvent::set_integer_value_field(
            event.as_deref(),
            CGEventField::EventSourceUserData,
            SYNTHETIC_EVENT_TAG,
        );
        CGEvent::post(
            CGEventTapLocation::AnnotatedSessionEventTap,
            event.as_deref(),
        );
    }
}
