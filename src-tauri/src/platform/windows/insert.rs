//! Pastes text into the focused field with the clipboard and Ctrl+V (or
//! Shift+Insert where the options ask for it, WIN-04), then puts the user's
//! clipboard back unless something else was copied meanwhile (INS-01..04).
//! An elevated app drops keys from a normal process, so the text stays on
//! the clipboard with an explanation instead (WIN-06).

use std::time::{Duration, Instant};

use handy_keys::Modifiers;

use super::keycode::{VK_INSERT, VK_RETURN};
use super::{clipboard, focus, hook, input, process};
use crate::platform::{
    CopyReason, Editable, InsertOptions, InsertOutcome, InsertText, PasteKey, PlatformError,
    SyntheticKey, TextInserter,
};

const VK_A: u16 = 0x41;
const VK_C: u16 = 0x43;
const VK_V: u16 = 0x56;
const VK_Z: u16 = 0x5A;
/// How long a copy may take to reach the clipboard.
const COPY_TIMEOUT: Duration = Duration::from_millis(400);
const COPY_POLL: Duration = Duration::from_millis(15);
/// How long to wait for the user to let go of Win, Alt and Shift first.
const RELEASE_WAIT: Duration = Duration::from_millis(400);

pub struct WinInserter;

impl WinInserter {
    fn copy_instead(
        &self,
        text: &InsertText,
        reason: CopyReason,
    ) -> Result<InsertOutcome, PlatformError> {
        self.copy(text)?;
        Ok(InsertOutcome::Copied(reason))
    }

    /// Puts the snapshot back after `hold`, unless something else was copied.
    fn restore_later(snapshot: clipboard::Snapshot, written: u32, hold: Duration) {
        std::thread::spawn(move || {
            std::thread::sleep(hold);
            if clipboard::sequence() != written {
                log::debug!("clipboard changed after paste; not restoring");
                return;
            }
            if let Err(error) = clipboard::restore(&snapshot) {
                log::warn!("could not restore the clipboard: {error}");
            }
        });
    }
}

/// Waits briefly until the user has let go of modifiers that would turn the
/// paste into another shortcut.
fn wait_for_release() {
    let deadline = Instant::now() + RELEASE_WAIT;
    while hook::held_modifiers().intersects(Modifiers::CMD | Modifiers::OPT | Modifiers::SHIFT)
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(15));
    }
}

fn paste(key: PasteKey) -> Result<(), PlatformError> {
    match key {
        PasteKey::Default => input::press(Modifiers::CTRL, VK_V),
        PasteKey::ShiftInsert => input::press(Modifiers::SHIFT, VK_INSERT),
    }
}

impl TextInserter for WinInserter {
    fn insert(
        &self,
        text: &InsertText,
        options: &InsertOptions,
    ) -> Result<InsertOutcome, PlatformError> {
        if process::foreground_pid().is_some_and(process::above_own_integrity) {
            return self.copy_instead(text, CopyReason::Elevated);
        }
        if options.require_field && focus::focused_field() == Editable::No {
            return self.copy_instead(text, CopyReason::NoTextField);
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
        let snapshot = clipboard::snapshot()?;
        wait_for_release();
        let mut written = 0;
        for (index, part) in parts.iter().enumerate() {
            if index > 0 {
                // Each part reaches the app as its own paste.
                std::thread::sleep(pause);
            }
            written = clipboard::write_text(part, true)?;
            paste(options.paste_key)?;
        }
        Self::restore_later(snapshot, written, options.hold);
        Ok(InsertOutcome::Pasted)
    }

    fn copy(&self, text: &InsertText) -> Result<(), PlatformError> {
        clipboard::write_text(text, false).map(|_| ())
    }

    fn copy_png(&self, png: &[u8]) -> Result<(), PlatformError> {
        clipboard::write_png(png)
    }

    fn press(&self, key: SyntheticKey, extra: Modifiers) -> Result<(), PlatformError> {
        match key {
            SyntheticKey::Enter => input::press(extra, VK_RETURN),
            SyntheticKey::SelectAll => input::press(Modifiers::CTRL, VK_A),
            SyntheticKey::Undo => input::press(Modifiers::CTRL, VK_Z),
        }
    }

    fn copy_selection(&self) -> Result<Option<String>, PlatformError> {
        let snapshot = clipboard::snapshot()?;
        let before = clipboard::sequence();
        input::press(Modifiers::CTRL, VK_C)?;
        let deadline = Instant::now() + COPY_TIMEOUT;
        let copied = loop {
            std::thread::sleep(COPY_POLL);
            if clipboard::sequence() != before {
                // Let the app finish writing every format.
                std::thread::sleep(COPY_POLL);
                break Some(clipboard::read_text().unwrap_or_default());
            }
            if Instant::now() >= deadline {
                break None;
            }
        };
        if copied.is_some() {
            clipboard::restore(&snapshot)?;
        }
        Ok(copied.filter(|text| !text.is_empty()))
    }
}
