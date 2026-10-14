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
mod login;
mod media;
mod mice;
mod mouse;
mod pasteboard;
mod permissions;
mod relaunch;
mod secure_input;
mod sound;

use std::sync::Arc;

use objc2_foundation::{NSLocale, NSString};

use super::{MainThread, Platform};
use crate::hotkeys::{macos_defaults, macos_label, macos_reserved};

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
        reserved: macos_reserved,
        language: preferred_language(),
        globe_key: globe::globe_key_action(),
        user_name: first_name(),
        login_item: Arc::new(login::MacLoginItem),
        region: region(),
        mice: Arc::new(mice::HidMice),
    }
}

/// Free space on the volume holding `path`, in bytes (APP-09).
pub fn free_bytes(path: &std::path::Path) -> Option<u64> {
    let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).ok()?;
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    let status = unsafe { libc::statvfs(path.as_ptr(), &mut stats) };
    (status == 0).then(|| stats.f_bavail as u64 * stats.f_frsize as u64)
}

/// The first word of the account's full name.
fn first_name() -> Option<String> {
    let full = objc2_foundation::NSFullUserName().to_string();
    full.split_whitespace().next().map(str::to_owned)
}

pub use focus::set_own_field_focused;
pub use globe::{open_keyboard_settings, open_sound_settings, open_storage_settings};
pub use relaunch::relaunch_after_exit;

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

/// The name of language `code` in `language`, e.g. "Französisch" for "fr"
/// in "de", as `Intl.DisplayNames` names it in the interface.
pub fn language_name(code: &str, language: &str) -> Option<String> {
    let locale = NSLocale::localeWithLocaleIdentifier(&NSString::from_str(language));
    let name = locale
        .localizedStringForLanguageCode(&NSString::from_str(code))?
        .to_string();
    let mut chars = name.chars();
    let first = chars.next()?;
    Some(first.to_uppercase().chain(chars).collect())
}

/// The region chosen in System Settings › General › Language & Region, e.g. "CA".
fn region() -> Option<String> {
    let code = NSLocale::currentLocale().regionCode()?.to_string();
    (!code.is_empty()).then_some(code)
}

/// The user's first preferred language as a BCP 47 tag, e.g. "en-GB".
fn preferred_language() -> Option<String> {
    let tag = NSLocale::preferredLanguages().firstObject()?.to_string();
    (!tag.is_empty()).then_some(tag)
}
