//! Rules for shortcuts the user records, with Wispr Flow's messages
//! (KEYS-03, KEYS-04, KEYS-05, KEYS-07): a modifier or a mouse button, at
//! most 3 keys, not reserved by the system, not used twice, and only one side
//! of each modifier. Caps Lock alone is for push to talk or hands-free, and
//! Esc alone for Cancel.

use handy_keys::{Key, Modifiers};

use super::trigger::{MODIFIER_GROUPS, Trigger};
use super::{Action, Binding};

const MAX_KEYS: usize = 3;

pub const BOTH_SIDES: &str = "Shortcut cannot contain both left and right version of the same key";
pub const IN_USE: &str = "This shortcut is already in use";
pub const TOO_MANY_KEYS: &str = "Shortcut must contain 3 or fewer keys";
pub const NEEDS_MODIFIER: &str = "Shortcut must include a modifier key or a valid mouse button";
pub const OVERLAP_WARNING: &str = "Warning: Your Hands-free mode shortcut is a part of a \
Push-to-talk shortcut. This may lead to conflicts.";

/// Shortcuts macOS or every app relies on.
fn reserved(trigger: &Trigger) -> bool {
    let Trigger::Keys(hotkey) = trigger else {
        return false;
    };
    let only = |modifiers: Modifiers| {
        MODIFIER_GROUPS
            .iter()
            .all(|group| hotkey.modifiers.intersects(*group) == modifiers.intersects(*group))
    };
    matches!(
        hotkey.key,
        Some(Key::Delete | Key::ForwardDelete | Key::Space | Key::C | Key::V)
    ) && only(Modifiers::CMD)
        || hotkey.key == Some(Key::F) && only(Modifiers::CMD | Modifiers::CTRL)
}

/// Letters, digits and punctuation type text, so they never stand alone.
fn types_text(key: Key) -> bool {
    let name = key.to_string();
    name.chars().count() == 1
        || matches!(
            key,
            Key::Num0
                | Key::Num1
                | Key::Num2
                | Key::Num3
                | Key::Num4
                | Key::Num5
                | Key::Num6
                | Key::Num7
                | Key::Num8
                | Key::Num9
                | Key::Minus
                | Key::Equal
                | Key::LeftBracket
                | Key::RightBracket
                | Key::Backslash
                | Key::Semicolon
                | Key::Quote
                | Key::Comma
                | Key::Period
                | Key::Slash
                | Key::Grave
                | Key::Section
        )
}

/// Checks `trigger` for `action`. `others` are every other binding in use,
/// including the action's own other bindings. `label` formats a shortcut
/// for the messages. The error is the message to show.
pub fn validate(
    trigger: &Trigger,
    action: Action,
    others: &[Binding],
    label: fn(&Trigger) -> String,
) -> Result<(), String> {
    let not_allowed = || format!("This shortcut is not allowed: {}", label(trigger));
    if trigger.key_count() > MAX_KEYS {
        return Err(TOO_MANY_KEYS.into());
    }
    if let Trigger::Keys(hotkey) = trigger
        && hotkey.modifiers.is_empty()
    {
        match hotkey.key {
            Some(Key::CapsLock) if matches!(action, Action::PushToTalk | Action::HandsFree) => {}
            Some(Key::Escape) if action == Action::Cancel => {}
            Some(Key::CapsLock | Key::Escape) => return Err(not_allowed()),
            Some(key) if types_text(key) => return Err(not_allowed()),
            _ => return Err(NEEDS_MODIFIER.into()),
        }
    }
    if reserved(trigger) {
        return Err(not_allowed());
    }
    if others.iter().any(|other| other.trigger.same_as(trigger)) {
        return Err(IN_USE.into());
    }
    Ok(())
}

/// Wispr's warning when a hands-free combination sits inside a
/// push-to-talk one (KEYS-07). The change still saves.
pub fn overlap_warning(bindings: &[Binding]) -> Option<&'static str> {
    let of = |action| {
        bindings
            .iter()
            .filter(move |binding: &&Binding| binding.action == action)
    };
    of(Action::HandsFree)
        .any(|free| {
            of(Action::PushToTalk).any(|talk| {
                !free.trigger.same_as(&talk.trigger) && free.trigger.is_part_of(&talk.trigger)
            })
        })
        .then_some(OVERLAP_WARNING)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkeys::{macos_defaults, macos_label};

    fn check(text: &str, action: Action) -> Result<(), String> {
        let others: Vec<Binding> = macos_defaults(true)
            .into_iter()
            .filter(|binding| binding.action != action)
            .collect();
        validate(&text.parse().unwrap(), action, &others, macos_label)
    }

    #[test]
    fn accepts_modifiers_mouse_buttons_and_modifier_only_chords() {
        assert!(check("Opt+Return", Action::PressEnter).is_ok());
        assert!(check("Ctrl+Opt", Action::PushToTalk).is_ok());
        assert!(check("Shift+F13", Action::PressEnter).is_ok());
        assert!(check("Mouse4", Action::PushToTalk).is_ok());
        assert!(check("Ctrl+MouseMiddle", Action::HandsFree).is_ok());
        assert!(check("Fn", Action::PushToTalk).is_ok());
    }

    #[test]
    fn standalone_keys_follow_wispr() {
        assert!(check("CapsLock", Action::PushToTalk).is_ok());
        assert!(check("CapsLock", Action::HandsFree).is_ok());
        assert_eq!(
            check("CapsLock", Action::PasteLast).unwrap_err(),
            "This shortcut is not allowed: Caps Lock"
        );
        assert!(check("Escape", Action::Cancel).is_ok());
        assert_eq!(
            check("Escape", Action::PushToTalk).unwrap_err(),
            "This shortcut is not allowed: Esc"
        );
        assert_eq!(
            check("K", Action::PressEnter).unwrap_err(),
            "This shortcut is not allowed: K"
        );
        assert_eq!(
            check("Return", Action::PressEnter).unwrap_err(),
            NEEDS_MODIFIER
        );
    }

    #[test]
    fn rejects_reserved_long_and_duplicate_shortcuts() {
        for reserved in ["Cmd+V", "Cmd+C", "Cmd+Space", "Cmd+Ctrl+F", "Cmd+Delete"] {
            assert!(
                check(reserved, Action::PressEnter)
                    .unwrap_err()
                    .starts_with("This shortcut is not allowed: "),
                "{reserved}"
            );
        }
        assert_eq!(
            check("Cmd+Space", Action::PressEnter).unwrap_err(),
            "This shortcut is not allowed: ⌘Space"
        );
        assert_eq!(
            check("Cmd+Ctrl+Opt+K", Action::PressEnter).unwrap_err(),
            TOO_MANY_KEYS
        );
        assert_eq!(check("Fn+Space", Action::PressEnter).unwrap_err(), IN_USE);
        assert_eq!(check("Cmd+Ctrl+V", Action::PushToTalk).unwrap_err(), IN_USE);
    }

    #[test]
    fn warns_when_hands_free_sits_inside_push_to_talk() {
        let binding = |action, text: &str| Binding::new(action, text.parse::<Trigger>().unwrap());
        assert_eq!(overlap_warning(&macos_defaults(true)), None);
        let overlapping = [
            binding(Action::PushToTalk, "Ctrl+Opt+Space"),
            binding(Action::HandsFree, "Ctrl+Opt"),
        ];
        assert_eq!(overlap_warning(&overlapping), Some(OVERLAP_WARNING));
    }
}
