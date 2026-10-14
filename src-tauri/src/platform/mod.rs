//! OS integration behind traits, so the dictation core stays portable and
//! testable with fakes. Only this module contains `#[cfg(target_os)]` code.

#[cfg(target_os = "macos")]
mod macos;
mod switchable;
#[cfg(target_os = "windows")]
mod windows;

pub use switchable::SwitchablePermissions;

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use handy_keys::Modifiers;

use crate::hotkeys::{Action, Binding, HotkeyEvent, Recording};
use crate::sounds::Cue;
use crate::t;

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("{}", t!("notice.accessibilityMissing"))]
    AccessibilityMissing,
    #[error("{0}")]
    Failed(String),
    #[error("not supported on this platform")]
    Unsupported,
}

/// Runs a closure on the app's main thread. AppKit pasteboard, input-source
/// and sound calls go through it.
pub type MainThread = Arc<dyn Fn(Box<dyn FnOnce() + Send>) + Send + Sync>;

/// Global shortcuts with press and release: keys, Fn and modifier-only
/// chords, and mouse buttons.
pub trait HotkeyBackend: Send {
    /// Starts the OS hooks; what they see is passed to `on_event` from the
    /// hooks' thread.
    fn start(
        &mut self,
        on_event: Box<dyn Fn(HotkeyEvent) + Send + Sync>,
    ) -> Result<(), PlatformError>;
    /// Replaces the active bindings. Bound input is swallowed, so it never reaches other apps.
    fn set_bindings(&mut self, bindings: &[Binding]);
    /// Actions a running dictation holds, which stuck-key recovery leaves
    /// alone (CORE-14).
    fn set_protected(&mut self, _actions: &[Action]) {}
    /// Starts recording a new shortcut, or stops with `None`. While
    /// recording, no shortcut fires and input is reported as
    /// `HotkeyEvent::Recording` (KEYS-01).
    fn set_recording(&mut self, _recording: Option<Recording>) {}
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
    /// The selection or app changed before Command Mode could replace it.
    TargetChanged,
    /// The app runs as administrator, so Windows drops Shrimp's keys (WIN-06).
    Elevated,
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

/// How a paste is made: how long the transcript stays on the clipboard,
/// whether a focused text field is required, and whether to paste in parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InsertOptions {
    pub hold: Duration,
    /// Leave the text on the clipboard when no text field is focused (INS-04).
    /// Terminals and remote desktops don't report their fields, so they skip
    /// the check (INS-10, INS-11).
    pub require_field: bool,
    /// Paste long text in parts (INS-09).
    pub split: Option<PasteSplit>,
    pub paste_key: PasteKey,
}

impl InsertOptions {
    pub fn new(hold: Duration) -> Self {
        Self {
            hold,
            require_field: true,
            split: None,
            paste_key: PasteKey::Default,
        }
    }
}

/// The keystroke that pastes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PasteKey {
    /// Cmd+V on macOS, Ctrl+V on Windows.
    #[default]
    Default,
    /// Shift+Insert, which Windows code editors, their terminals and terminal
    /// apps treat as a plain paste, where Ctrl+V can mean something else (Vim
    /// keys, mintty, PuTTY, the legacy console) (WIN-04). macOS ignores it.
    ShiftInsert,
}

/// Limits for each part of a split paste, and the pause between parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasteSplit {
    pub max_chars: usize,
    pub max_breaks: usize,
    pub pause: Duration,
}

impl PasteSplit {
    /// Cuts `text` into parts of at most `max_chars` characters and
    /// `max_breaks` line breaks, at line breaks or spaces where possible.
    /// Joined back together, the parts are exactly `text`.
    pub fn parts(&self, text: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut rest = text;
        while !rest.is_empty() {
            let end = self.part_end(rest);
            parts.push(rest[..end].to_owned());
            rest = &rest[end..];
        }
        parts
    }

    fn part_end(&self, text: &str) -> usize {
        let mut breaks = 0;
        let mut last_space = None;
        for (count, (index, c)) in text.char_indices().enumerate() {
            if count == self.max_chars.max(1) {
                return last_space.unwrap_or(index);
            }
            if c == '\n' {
                breaks += 1;
                if breaks == self.max_breaks.max(1) {
                    return index + 1;
                }
            }
            if c.is_whitespace() {
                last_space = Some(index + c.len_utf8());
            }
        }
        text.len()
    }
}

/// Keys Shrimp presses in the focused app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticKey {
    Enter,
    SelectAll,
    Undo,
}

pub trait TextInserter: Send + Sync {
    /// Pastes `text` into the focused field: snapshot the clipboard, write the
    /// text, send the paste keystroke and restore the clipboard after
    /// `options.hold` unless something else was copied meanwhile.
    fn insert(
        &self,
        text: &InsertText,
        options: &InsertOptions,
    ) -> Result<InsertOutcome, PlatformError>;
    /// Puts `text` on the clipboard and leaves it there.
    fn copy(&self, text: &InsertText) -> Result<(), PlatformError>;
    /// Puts a PNG image on the clipboard (Insights' share card, STA-03).
    fn copy_png(&self, _png: &[u8]) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    /// Presses `key` in the focused app with `extra` modifiers held (Shift+Enter).
    fn press(&self, _key: SyntheticKey, _extra: Modifiers) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    /// Copies the selection of the focused app with the copy shortcut and
    /// returns it, putting the clipboard back afterwards (CMD-02 fallback).
    fn copy_selection(&self) -> Result<Option<String>, PlatformError> {
        Err(PlatformError::Unsupported)
    }
}

/// The app a dictation is for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppIdentity {
    /// Bundle ID on macOS, executable name on Windows.
    pub bundle_id: String,
    pub name: String,
}

/// A span of a field's text in UTF-16 code units, as Accessibility counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextRange {
    pub start: usize,
    pub len: usize,
}

impl TextRange {
    pub fn end(&self) -> usize {
        self.start + self.len
    }
}

/// What kind of field has focus, as far as reading and pasting go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldKind {
    #[default]
    Text,
    /// A code editor (Monaco in Cursor, VS Code and Windsurf).
    Code,
    /// A terminal, including IDE terminals.
    Terminal,
    /// A password or other secure field. Its text is never read (CTX-03).
    Secure,
}

/// Whether the focused element takes typed text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Editable {
    Yes,
    No,
    /// The app doesn't say; pasting is worth a try.
    #[default]
    Unknown,
}

/// The focused field and what surrounds it, read through Accessibility.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldContext {
    /// Identifies the element, so a later read can tell it is the same field.
    pub field_id: u64,
    pub kind: FieldKind,
    pub editable: Editable,
    /// The selection, or the cursor when its length is 0.
    pub selection: Option<TextRange>,
    pub selected_text: String,
    /// Text just before the selection and just after it.
    pub before: String,
    pub after: String,
    /// The whole text, when the request's limit allows.
    pub text: Option<String>,
    /// Length of the field's text in UTF-16 units.
    pub length: usize,
    /// The web page the field is in, or the page the window shows.
    pub url: Option<String>,
    /// The field is part of a web page; browser address bars are not.
    pub in_web_content: bool,
    pub window_title: Option<String>,
    /// Short texts visible in the focused window: labels, names, file names.
    pub screen_text: Vec<String>,
    /// The text of code editors in the window, for identifiers (DEV-02).
    pub code_text: Vec<String>,
}

/// How much of the focused field and window to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContextRequest {
    /// Characters to read on each side of the selection.
    pub around: usize,
    /// Read the whole text when it has at most this many characters (0: never).
    pub text_limit: usize,
    /// Collect the short texts of the focused window.
    pub scan_window: bool,
    /// Look for the web page the window shows even when no field is focused.
    pub find_url: bool,
}

/// What has focus: the frontmost app, and its focused field.
pub trait FocusInspector: Send + Sync {
    fn frontmost_app(&self) -> Option<AppIdentity>;
    /// Reads the focused field of the frontmost app. Blocking, with a short
    /// timeout per call. Secure fields and Shrimp's own windows are never read.
    fn read_field(&self, _request: &ContextRequest) -> Option<FieldContext> {
        None
    }
    /// Selects `range` in the focused field, if it is still `field_id`.
    fn select_range(&self, _field_id: u64, _range: TextRange) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    Microphone,
    Accessibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Puts back what [`MediaControl`] changed.
pub type Restore = Box<dyn FnOnce() + Send>;

/// Quiets playing audio while dictating (SND-03, SND-04).
pub trait MediaControl: Send + Sync {
    /// Mutes the output if something is playing; the closure unmutes it.
    fn mute(&self) -> Option<Restore>;
    /// Pauses the playing media app unless it is a call; the closure
    /// resumes it if it is still paused.
    fn pause(&self) -> Option<Restore>;
}

/// Launch at login (APP-06).
pub trait LoginItem: Send + Sync {
    fn enabled(&self) -> bool;
    fn set_enabled(&self, on: bool) -> Result<(), PlatformError>;
    /// This launch came from the login item, so Shrimp starts in the background.
    fn launched_at_login(&self) -> bool;
}

/// Opens URLs in the default browser (CMD-05 web searches).
pub trait Launcher: Send + Sync {
    fn open_url(&self, url: &str) -> Result<(), PlatformError>;
}

/// The OS services the app uses, built once at startup.
pub struct Platform {
    pub hotkeys: Box<dyn HotkeyBackend>,
    pub inserter: Arc<dyn TextInserter>,
    pub focus: Arc<dyn FocusInspector>,
    pub permissions: Arc<dyn Permissions>,
    pub sounds: Arc<dyn CuePlayer>,
    pub launcher: Arc<dyn Launcher>,
    /// Input devices (MIC-02..08).
    pub devices: Arc<dyn crate::audio::DeviceProvider>,
    pub media: Arc<dyn MediaControl>,
    pub default_bindings: Vec<Binding>,
    /// Formats a binding for display, e.g. "fn Space" or "⌃⌘V".
    pub label: fn(&crate::hotkeys::Trigger) -> String,
    /// The platform's own shortcuts, which can't be recorded (KEYS-05, WIN-03).
    pub reserved: fn(&crate::hotkeys::Trigger) -> bool,
    /// The user's preferred language as a BCP 47 tag ("en-GB"), the first
    /// language to dictate in.
    pub language: Option<String>,
    /// What the Fn key does by itself, which a Fn shortcut must not trigger.
    pub globe_key: GlobeKeyAction,
    /// The user's first name, for Home's greeting (STA-02).
    pub user_name: Option<String>,
    pub login_item: Arc<dyn LoginItem>,
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
    pub fn conflict(self) -> Option<String> {
        match self {
            Self::DoNothing => None,
            Self::ChangeInputSource => Some(t!("notice.globeKey.inputSource")),
            Self::ShowEmoji => Some(t!("notice.globeKey.emoji")),
            Self::StartDictation => Some(t!("notice.globeKey.dictation")),
        }
    }
}

#[cfg(target_os = "macos")]
pub fn native(main_thread: MainThread) -> Platform {
    macos::platform(main_thread)
}

#[cfg(target_os = "windows")]
pub fn native(main_thread: MainThread) -> Platform {
    windows::platform(main_thread)
}

#[cfg(target_os = "macos")]
pub use macos::{
    free_bytes, language_name, open_keyboard_settings, open_sound_settings, open_storage_settings,
    relaunch_after_exit, set_own_field_focused,
};

#[cfg(target_os = "windows")]
pub use windows::{
    free_bytes, language_name, open_keyboard_settings, open_sound_settings, open_storage_settings,
    relaunch_after_exit, set_own_field_focused,
};

/// What a restart passes the new instance, so it opens its main window even
/// when the launch looks like one at login (APP-12).
pub const RELAUNCH_ARG: &str = "--relaunched";

/// The shortcut hints of the tray menu: paste last, copy last and Quit.
pub const TRAY_ACCELERATORS: [Option<&str>; 3] = if cfg!(target_os = "windows") {
    [Some("Shift+Alt+Z"), Some("Shift+Alt+X"), None]
} else {
    [Some("Ctrl+Cmd+V"), Some("Ctrl+Cmd+C"), Some("CmdOrCtrl+Q")]
};

/// Whether a left click on the tray icon opens the main window, with the
/// menu on a right click: Windows' notification area (Wispr Flow opens its
/// Hub, article 2503460374). The macOS menu bar icon opens the menu.
pub const TRAY_CLICK_OPENS_WINDOW: bool = cfg!(target_os = "windows");

/// Whether "Mute while dictating" starts on: off on macOS, on on Windows
/// (SND-03, WIN-08).
pub const MUTE_BY_DEFAULT: bool = cfg!(target_os = "windows");

/// Which interface texts replace the macOS wording ("this Mac", "System
/// Settings"), if any.
pub const TEXT_VARIANT: Option<&str> = if cfg!(target_os = "windows") {
    Some("windows")
} else {
    None
};

/// Adds the Tauri plugins the platform backend relies on.
pub fn register_plugins<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(macos::flow_bar::plugin());
    builder
}

#[cfg(target_os = "macos")]
pub use macos::flow_bar;

#[cfg(target_os = "windows")]
pub use windows::flow_bar;

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
    fn split_pastes_keep_every_character_within_the_limits() {
        let split = PasteSplit {
            max_chars: 12,
            max_breaks: 2,
            pause: Duration::ZERO,
        };
        let text = "one two three four five six\nseven\neight\nnine ten elevenelevenEleven";
        let parts = split.parts(text);
        assert_eq!(parts.concat(), text);
        assert!(parts.len() > 3);
        for part in &parts {
            assert!(part.chars().count() <= 12, "{part:?}");
            assert!(part.matches('\n').count() <= 2, "{part:?}");
        }
        assert_eq!(parts[0], "one two ", "cut at a space");
        assert_eq!(split.parts("short"), ["short"]);
        assert!(split.parts("").is_empty());
    }

    #[test]
    fn only_do_nothing_is_free_of_conflict() {
        assert_eq!(GlobeKeyAction::DoNothing.conflict(), None);
        assert_eq!(
            GlobeKeyAction::ShowEmoji.conflict().as_deref(),
            Some("opens the emoji picker")
        );
    }
}
