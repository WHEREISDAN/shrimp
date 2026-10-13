//! macOS backend. Permissions: Accessibility for the keyboard hook and for
//! posting Cmd+V; Microphone for live capture.

mod ax;
mod context;
mod core_audio;
pub mod flow_bar;
mod focus;
mod globe;
mod hotkeys;
mod insert;
mod iokit;
mod keymap;
mod launcher;
mod media;
mod mouse;
mod pasteboard;
mod permissions;
mod secure_input;
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
        launcher: Arc::new(launcher::MacLauncher),
        devices: Arc::new(core_audio::CoreAudioDevices),
        media: Arc::new(media::MacMedia),
        default_bindings: macos_defaults(has_fn_key()),
        label: macos_label,
        language: preferred_language(),
        globe_key: globe::globe_key_action(),
        user_name: first_name(),
    }
}

/// The first word of the account's full name.
fn first_name() -> Option<String> {
    let full = objc2_foundation::NSFullUserName().to_string();
    full.split_whitespace().next().map(str::to_owned)
}

pub use globe::{open_keyboard_settings, open_sound_settings};

/// Debug builds read this variable to pretend the keyboard has (1) or
/// lacks (0) an Apple Fn key. See docs/DEVELOPMENT.md.
const FN_KEY_ENV: &str = "SHRIMP_FN_KEY";

/// Whether the defaults can use Fn (CORE-10).
fn has_fn_key() -> bool {
    let forced = cfg!(debug_assertions)
        .then(|| std::env::var(FN_KEY_ENV).ok())
        .flatten();
    let has = match forced.as_deref() {
        Some("0") => false,
        Some("1") => true,
        _ => iokit::has_apple_fn_key(),
    };
    log::info!("Apple Fn key: {has}");
    has
}

/// The user's first preferred language as a BCP 47 tag, e.g. "en-GB".
fn preferred_language() -> Option<String> {
    let tag = NSLocale::preferredLanguages().firstObject()?.to_string();
    (!tag.is_empty()).then_some(tag)
}
