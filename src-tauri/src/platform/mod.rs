//! OS integration behind traits, so the dictation core stays portable and
//! testable with fakes. Only this module contains `#[cfg(target_os)]` code.

#[cfg(target_os = "macos")]
mod macos;

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::hotkeys::{Binding, ShortcutEvent};
use crate::sounds::Cue;

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("Shrimp needs Accessibility permission")]
    AccessibilityMissing,
    #[error("{0}")]
    Failed(String),
    #[error("not supported on this platform")]
    Unsupported,
}

/// Runs a closure on the app's main thread. AppKit pasteboard, input-source
/// and sound calls go through it.
pub type MainThread = Arc<dyn Fn(Box<dyn FnOnce() + Send>) + Send + Sync>;

/// Global shortcuts, including Fn and modifier-only chords, with press and release.
pub trait HotkeyBackend: Send {
    /// Starts the OS keyboard hook; matched shortcuts are passed to `on_event`
    /// from the hook's thread.
    fn start(
        &mut self,
        on_event: Box<dyn Fn(ShortcutEvent) + Send + Sync>,
    ) -> Result<(), PlatformError>;
    /// Replaces the active bindings. Bound input is swallowed, so it never reaches other apps.
    fn set_bindings(&mut self, bindings: &[Binding]);
    fn stop(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertOutcome {
    Pasted,
    /// The text was left on the clipboard instead (INS-04).
    Copied(CopyReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyReason {
    NoTextField,
    AccessibilityMissing,
}

/// Text to paste: plain always, HTML too when it has lists or formatting.
/// Apps that accept rich text take the HTML; the rest take the plain text.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InsertText {
    pub plain: String,
    pub html: Option<String>,
}

impl InsertText {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            plain: text.into(),
            html: None,
        }
    }
}

pub trait TextInserter: Send + Sync {
    /// Pastes `text` into the focused field: snapshot the clipboard, write the
    /// text, send the paste keystroke and restore the clipboard after `hold`
    /// unless something else was copied meanwhile.
    fn insert(&self, text: &InsertText, hold: Duration) -> Result<InsertOutcome, PlatformError>;
    /// Puts `text` on the clipboard and leaves it there.
    fn copy(&self, text: &InsertText) -> Result<(), PlatformError>;
}

/// The app a dictation is for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppIdentity {
    /// Bundle ID on macOS, executable name on Windows.
    pub bundle_id: String,
    pub name: String,
}

/// What has focus. Phase 3 adds the selection and the text around the cursor.
pub trait FocusInspector: Send + Sync {
    fn frontmost_app(&self) -> Option<AppIdentity>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    Microphone,
    Accessibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionStatus {
    Granted,
    Denied,
    /// Never asked yet; the OS prompts on first use.
    NotDetermined,
}

pub trait Permissions: Send + Sync {
    fn status(&self, permission: Permission) -> PermissionStatus;
    /// Shows the OS prompt where one exists, otherwise the settings pane.
    fn request(&self, permission: Permission) -> Result<(), PlatformError>;
    fn open_settings(&self, permission: Permission) -> Result<(), PlatformError>;
}

pub trait CuePlayer: Send + Sync {
    fn play(&self, cue: Cue);
}

/// The OS services the app uses, built once at startup.
pub struct Platform {
    pub hotkeys: Box<dyn HotkeyBackend>,
    pub inserter: Arc<dyn TextInserter>,
    pub focus: Arc<dyn FocusInspector>,
    pub permissions: Arc<dyn Permissions>,
    pub sounds: Arc<dyn CuePlayer>,
    pub default_bindings: Vec<Binding>,
    /// Formats a binding for display, e.g. "fn Space" or "⌃⌘V".
    pub label: fn(&handy_keys::Hotkey) -> String,
    /// The user's preferred language (ISO 639-1), the dictation language
    /// until the language settings arrive.
    pub language: Option<String>,
    /// What the Fn key does by itself, which a Fn shortcut must not trigger.
    pub globe_key: GlobeKeyAction,
}

/// The system action of the Fn (Globe) key pressed alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobeKeyAction {
    DoNothing,
    ChangeInputSource,
    ShowEmoji,
    StartDictation,
}

impl GlobeKeyAction {
    /// macOS `AppleFnUsageType`; when unset, Macs with a Globe key show emoji.
    pub fn from_usage_type(value: Option<isize>) -> Self {
        match value {
            Some(0) => Self::DoNothing,
            Some(1) => Self::ChangeInputSource,
            Some(3) => Self::StartDictation,
            _ => Self::ShowEmoji,
        }
    }

    /// Why holding Fn to dictate would also trigger a system action (CORE-02).
    pub fn conflict(self) -> Option<&'static str> {
        match self {
            Self::DoNothing => None,
            Self::ChangeInputSource => Some("switches the input source"),
            Self::ShowEmoji => Some("opens the emoji picker"),
            Self::StartDictation => Some("starts macOS dictation"),
        }
    }
}

#[cfg(target_os = "macos")]
pub fn native(main_thread: MainThread) -> Platform {
    macos::platform(main_thread)
}

#[cfg(target_os = "macos")]
pub use macos::open_keyboard_settings;

/// Adds the Tauri plugins the platform backend relies on.
pub fn register_plugins<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(macos::flow_bar::plugin());
    builder
}

#[cfg(target_os = "macos")]
pub use macos::flow_bar;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_globe_key_setting() {
        assert_eq!(
            GlobeKeyAction::from_usage_type(Some(0)),
            GlobeKeyAction::DoNothing
        );
        assert_eq!(
            GlobeKeyAction::from_usage_type(Some(1)),
            GlobeKeyAction::ChangeInputSource
        );
        assert_eq!(
            GlobeKeyAction::from_usage_type(Some(2)),
            GlobeKeyAction::ShowEmoji
        );
        assert_eq!(
            GlobeKeyAction::from_usage_type(Some(3)),
            GlobeKeyAction::StartDictation
        );
        assert_eq!(
            GlobeKeyAction::from_usage_type(None),
            GlobeKeyAction::ShowEmoji
        );
    }

    #[test]
    fn only_do_nothing_is_free_of_conflict() {
        assert_eq!(GlobeKeyAction::DoNothing.conflict(), None);
        assert_eq!(
            GlobeKeyAction::ShowEmoji.conflict(),
            Some("opens the emoji picker")
        );
    }
}
