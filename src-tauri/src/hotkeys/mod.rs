//! Shortcut bindings and matching, independent of the OS keyboard hook.

mod binding;
mod matcher;
pub mod validation;

pub use binding::{
    Action, Binding, backspace_cancel, blocked_hotkeys, macos_defaults, macos_label,
};
pub use matcher::{Matcher, ShortcutEvent};
