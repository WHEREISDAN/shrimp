//! Shortcut bindings and matching, independent of the OS keyboard hook.

mod binding;
mod matcher;

pub use binding::{Action, Binding, macos_defaults, macos_label};
pub use matcher::{Matcher, ShortcutEvent};
