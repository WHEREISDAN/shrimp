//! macOS backend. Permissions: Accessibility for the keyboard hook and for
//! posting Cmd+V; Microphone for live capture.

pub mod flow_bar;
mod focus;
mod globe;
mod hotkeys;
mod insert;
mod keymap;
mod pasteboard;
mod permissions;
mod sound;

use std::sync::Arc;

use objc2_foundation::NSLocale;

use super::{MainThread, Platform};
use crate::hotkeys::{macos_defaults, macos_label};

pub fn platform(main_thread: MainThread) -> Platform {
    Platform {
        hotkeys: Box::new(hotkeys::MacHotkeys::default()),
        inserter: Arc::new(insert::MacInserter::new(Arc::clone(&main_thread))),
        focus: Arc::new(focus::MacFocus),
        permissions: Arc::new(permissions::MacPermissions),
        sounds: Arc::new(sound::MacCuePlayer::new(main_thread)),
        default_bindings: macos_defaults(),
        label: macos_label,
        language: preferred_language(),
        globe_key: globe::globe_key_action(),
    }
}

pub use globe::open_keyboard_settings;

/// The user's first preferred language as an ISO 639-1 code, e.g. "en".
fn preferred_language() -> Option<String> {
    let tag = NSLocale::preferredLanguages().firstObject()?.to_string();
    tag.split(['-', '_'])
        .next()
        .filter(|code| !code.is_empty())
        .map(str::to_lowercase)
}
