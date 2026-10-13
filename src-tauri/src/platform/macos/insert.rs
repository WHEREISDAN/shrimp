//! Pastes text into the focused field with the clipboard and Cmd+V, then puts
//! the user's clipboard back (INS-01..04). Also presses Enter, Select All and
//! Undo, and copies the selection when Accessibility can't read it (CMD-02).

use std::sync::mpsc;
use std::time::{Duration, Instant};

use handy_keys::Modifiers;
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation,
};

use super::focus;
use super::{keymap, pasteboard, permissions};
use crate::platform::{
    CopyReason, Editable, InsertOptions, InsertOutcome, InsertText, MainThread, PlatformError,
    SyntheticKey, TextInserter,
};

/// Marks key events Shrimp posts itself ("SHRP"), so they can be told apart
/// from the user's own typing.
pub const SYNTHETIC_EVENT_TAG: i64 = 0x5348_5250;

const MAIN_THREAD_TIMEOUT: Duration = Duration::from_secs(2);
/// `kVK_Return`, the same key on every layout.
const RETURN_KEY: u16 = 36;
/// How long a copy may take to reach the clipboard.
const COPY_TIMEOUT: Duration = Duration::from_millis(400);
const COPY_POLL: Duration = Duration::from_millis(15);

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

    fn copy_instead(
        &self,
        text: &InsertText,
        reason: CopyReason,
    ) -> Result<InsertOutcome, PlatformError> {
        self.copy(text)?;
        Ok(InsertOutcome::Copied(reason))
    }

    /// Puts the snapshot back after `hold`, unless something else was copied.
    fn restore_later(&self, snapshot: pasteboard::Snapshot, written: isize, hold: Duration) {
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
    }
}

impl TextInserter for MacInserter {
    fn insert(
        &self,
        text: &InsertText,
        options: &InsertOptions,
    ) -> Result<InsertOutcome, PlatformError> {
        if !permissions::accessibility_trusted() {
            return self.copy_instead(text, CopyReason::AccessibilityMissing);
        }
        if options.require_field {
            let field = focus::frontmost_pid().map_or(Editable::Unknown, focus::focused_field);
            if field == Editable::No {
                return self.copy_instead(text, CopyReason::NoTextField);
            }
        }
        let parts: Vec<InsertText> = match options.split {
            Some(split) if text.html.is_none() => split
                .parts(&text.plain)
                .into_iter()
                .map(InsertText::plain)
                .collect(),
            _ => vec![text.clone()],
        };
        let pause = options.split.map_or(Duration::ZERO, |split| split.pause);
        let mut parts = parts.into_iter();
        let first = parts.next().unwrap_or_default();
        let (snapshot, mut written) = self.on_main(move || {
            let snapshot = pasteboard::snapshot();
            let written = pasteboard::write_text(&first, true);
            post_command_key(keymap::paste_key_code());
            (snapshot, written)
        })?;
        for part in parts {
            // Each part reaches the app as its own paste.
            std::thread::sleep(pause);
            written = self.on_main(move || {
                let written = pasteboard::write_text(&part, true);
                post_command_key(keymap::paste_key_code());
                written
            })?;
        }
        self.restore_later(snapshot, written, options.hold);
        Ok(InsertOutcome::Pasted)
    }

    fn copy(&self, text: &InsertText) -> Result<(), PlatformError> {
        let text = text.clone();
        self.on_main(move || {
            pasteboard::write_text(&text, false);
        })
    }

    fn press(&self, key: SyntheticKey, extra: Modifiers) -> Result<(), PlatformError> {
        if !permissions::accessibility_trusted() {
            return Err(PlatformError::AccessibilityMissing);
        }
        let flags = event_flags(extra);
        self.on_main(move || match key {
            SyntheticKey::Enter => post_key(RETURN_KEY, flags),
            SyntheticKey::SelectAll => {
                post_command_key(keymap::command_key_code('a', keymap::ANSI_A));
            }
            SyntheticKey::Undo => post_command_key(keymap::command_key_code('z', keymap::ANSI_Z)),
        })
    }

    fn copy_selection(&self) -> Result<Option<String>, PlatformError> {
        if !permissions::accessibility_trusted() {
            return Err(PlatformError::AccessibilityMissing);
        }
        let (snapshot, before) = self.on_main(|| {
            let snapshot = pasteboard::snapshot();
            let before = pasteboard::change_count();
            post_command_key(keymap::command_key_code('c', keymap::ANSI_C));
            (snapshot, before)
        })?;
        let deadline = Instant::now() + COPY_TIMEOUT;
        let copied = loop {
            std::thread::sleep(COPY_POLL);
            let (count, text) =
                self.on_main(|| (pasteboard::change_count(), pasteboard::read_text()))?;
            if count != before {
                break Some(text.unwrap_or_default());
            }
            if Instant::now() >= deadline {
                break None;
            }
        };
        if copied.is_some() {
            self.on_main(move || pasteboard::restore(&snapshot))?;
        }
        Ok(copied.filter(|text| !text.is_empty()))
    }
}

fn event_flags(modifiers: Modifiers) -> CGEventFlags {
    [
        (Modifiers::SHIFT, CGEventFlags::MaskShift),
        (Modifiers::CTRL, CGEventFlags::MaskControl),
        (Modifiers::OPT, CGEventFlags::MaskAlternate),
        (Modifiers::CMD, CGEventFlags::MaskCommand),
    ]
    .into_iter()
    .filter(|(modifier, _)| modifiers.intersects(*modifier))
    .fold(CGEventFlags::empty(), |flags, (_, flag)| flags | flag)
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
