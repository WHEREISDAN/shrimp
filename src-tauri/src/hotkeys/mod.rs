//! Shortcut bindings, matching and recording, independent of the OS hooks.

mod binding;
mod matcher;
pub mod recording;
mod trigger;
pub mod validation;

pub use binding::{
    Action, Binding, backspace_cancel, blocked_buttons, blocked_hotkeys, macos_defaults,
    macos_label, macos_reserved, windows_defaults, windows_label, windows_reserved,
};
pub use matcher::{Matched, Matcher, MouseEvent, ShortcutEvent};
pub use recording::{RecordInput, RecordUpdate, Recorded, Recording};
pub use trigger::{MouseButton, Trigger};

/// What the OS hooks report to the dictation controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyEvent {
    Shortcut(ShortcutEvent),
    /// A key that is no shortcut went down during a held shortcut (CORE-12).
    OtherKey,
    /// Progress of a shortcut being recorded (KEYS-01).
    Recording(RecordUpdate),
    /// Another app turned Secure Keyboard Entry on (its name) or off (KEYS-09).
    SecureInput(Option<String>),
}
