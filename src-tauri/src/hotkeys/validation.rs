//! Rules for shortcuts the user records (part of KEYS-03 and KEYS-05): a
//! modifier, at most 3 keys, not reserved by the system, and not already
//! used by another action. Modifiers match either side.

use handy_keys::{Hotkey, Key, Modifiers};

use super::{Action, Binding};

const MAX_KEYS: usize = 3;
const GROUPS: [Modifiers; 5] = [
    Modifiers::CMD,
    Modifiers::SHIFT,
    Modifiers::CTRL,
    Modifiers::OPT,
    Modifiers::FN,
];

/// Shortcuts macOS or every app relies on.
fn reserved(hotkey: &Hotkey) -> bool {
    let only = |modifiers: Modifiers| {
        GROUPS
            .iter()
            .all(|group| hotkey.modifiers.intersects(*group) == modifiers.intersects(*group))
    };
    matches!(
        hotkey.key,
        Some(Key::Delete | Key::ForwardDelete | Key::Space | Key::C | Key::V)
    ) && only(Modifiers::CMD)
        || hotkey.key == Some(Key::F) && only(Modifiers::CMD | Modifiers::CTRL)
}

/// Parses and checks `text` ("Opt+Return") as a shortcut for `action`.
/// `taken` are the other active bindings. The error is the message to show.
pub fn validate(text: &str, action: Action, taken: &[Binding]) -> Result<Hotkey, String> {
    let hotkey: Hotkey = text
        .parse()
        .map_err(|_| "That shortcut can't be used.".to_owned())?;
    if hotkey.modifiers.is_empty() {
        return Err("Add a modifier key such as ⌃, ⌥, ⇧ or ⌘.".into());
    }
    let modifiers = GROUPS
        .iter()
        .filter(|group| hotkey.modifiers.intersects(**group))
        .count();
    if hotkey.key.is_none() && modifiers < 2 {
        return Err("Add a key or a second modifier.".into());
    }
    let keys = modifiers + usize::from(hotkey.key.is_some());
    if keys > MAX_KEYS {
        return Err(format!("Use at most {MAX_KEYS} keys."));
    }
    if reserved(&hotkey) {
        return Err("This shortcut is not allowed".into());
    }
    if let Some(other) = taken.iter().find(|binding| {
        binding.action != action
            && binding.hotkey.key == hotkey.key
            && binding.hotkey.modifiers.matches(hotkey.modifiers)
    }) {
        return Err(format!("Already used for {}.", other.action.label()));
    }
    Ok(hotkey)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkeys::macos_defaults;

    fn check(text: &str) -> Result<Hotkey, String> {
        validate(text, Action::PressEnter, &macos_defaults())
    }

    #[test]
    fn accepts_a_modifier_and_a_key() {
        assert!(check("Opt+Return").is_ok());
        assert!(check("Ctrl+Opt").is_ok());
        assert!(check("Shift+F13").is_ok());
    }

    #[test]
    fn rejects_what_the_rules_forbid() {
        assert_eq!(
            check("Return").unwrap_err(),
            "Add a modifier key such as ⌃, ⌥, ⇧ or ⌘."
        );
        assert_eq!(check("Cmd+V").unwrap_err(), "This shortcut is not allowed");
        assert_eq!(
            check("Cmd+Space").unwrap_err(),
            "This shortcut is not allowed"
        );
        assert_eq!(
            check("Cmd+Ctrl+F").unwrap_err(),
            "This shortcut is not allowed"
        );
        assert_eq!(check("Cmd+Ctrl+Opt+K").unwrap_err(), "Use at most 3 keys.");
        assert_eq!(
            check("Fn+Space").unwrap_err(),
            "Already used for Hands-free mode."
        );
        assert_eq!(
            check("Cmd+Ctrl+V").unwrap_err(),
            "Already used for Paste last transcript."
        );
        assert!(check("not a key+++").is_err());
        assert_eq!(
            check("Ctrl").unwrap_err(),
            "Add a key or a second modifier."
        );
    }
}
