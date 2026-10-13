//! Rules for shortcuts the user records, with Wispr Flow's messages
//! (KEYS-03, KEYS-04, KEYS-05, KEYS-07): a modifier or a mouse button, at
//! most 3 keys, not reserved by the system, not used twice, and only one side
//! of each modifier. Caps Lock alone is for push to talk or hands-free, and
//! Esc alone for Cancel.

use handy_keys::Key;

use super::trigger::Trigger;
use super::{Action, Binding};
use crate::t;

const MAX_KEYS: usize = 3;

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
/// for the messages, and `reserved` tells the platform's own shortcuts
/// (KEYS-05, WIN-03). The error is the message to show.
pub fn validate(
    trigger: &Trigger,
    action: Action,
    others: &[Binding],
    label: fn(&Trigger) -> String,
    reserved: fn(&Trigger) -> bool,
) -> Result<(), String> {
    let not_allowed = || t!("error.hotkeys.notAllowed", shortcut = label(trigger));
    if trigger.key_count() > MAX_KEYS {
        return Err(t!("error.hotkeys.tooManyKeys", max = MAX_KEYS));
    }
    if let Trigger::Keys(hotkey) = trigger
        && hotkey.modifiers.is_empty()
    {
        match hotkey.key {
            Some(Key::CapsLock) if matches!(action, Action::PushToTalk | Action::HandsFree) => {}
            Some(Key::Escape) if action == Action::Cancel => {}
            Some(Key::CapsLock | Key::Escape) => return Err(not_allowed()),
            Some(key) if types_text(key) => return Err(not_allowed()),
            _ => return Err(t!("error.hotkeys.needsModifier")),
        }
    }
    if reserved(trigger) {
        return Err(not_allowed());
    }
    if others.iter().any(|other| other.trigger.same_as(trigger)) {
        return Err(t!("error.hotkeys.inUse"));
    }
    Ok(())
}

/// Wispr's warning when a hands-free combination sits inside a
/// push-to-talk one (KEYS-07). The change still saves.
pub fn overlap_warning(bindings: &[Binding]) -> Option<String> {
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
        .then(|| t!("error.hotkeys.overlap"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkeys::{
        macos_defaults, macos_label, macos_reserved, windows_defaults, windows_label,
        windows_reserved,
    };

    fn check(text: &str, action: Action) -> Result<(), String> {
        let others: Vec<Binding> = macos_defaults(true)
            .into_iter()
            .filter(|binding| binding.action != action)
            .collect();
        validate(
            &text.parse().unwrap(),
            action,
            &others,
            macos_label,
            macos_reserved,
        )
    }

    fn check_windows(text: &str, action: Action) -> Result<(), String> {
        let others: Vec<Binding> = windows_defaults()
            .into_iter()
            .filter(|binding| binding.action != action)
            .collect();
        validate(
            &text.parse().unwrap(),
            action,
            &others,
            windows_label,
            windows_reserved,
        )
    }

    #[test]
    fn windows_rejects_its_reserved_shortcuts() {
        assert_eq!(
            check_windows("Ctrl", Action::PushToTalk).unwrap_err(),
            "This shortcut is not allowed: Ctrl"
        );
        assert_eq!(
            check_windows("Win+E", Action::PressEnter).unwrap_err(),
            "This shortcut is not allowed: Win+E"
        );
        assert_eq!(
            check_windows("Ctrl+V", Action::PressEnter).unwrap_err(),
            "This shortcut is not allowed: Ctrl+V"
        );
        assert!(check_windows("Ctrl+Alt", Action::HandsFree).is_ok());
        assert_eq!(
            check_windows("Shift+Alt+Z", Action::PressEnter).unwrap_err(),
            "This shortcut is already in use"
        );
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
            "Shortcut must include a modifier key or a valid mouse button"
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
            "Shortcut must contain 3 or fewer keys"
        );
        assert_eq!(
            check("Fn+Space", Action::PressEnter).unwrap_err(),
            "This shortcut is already in use"
        );
        assert_eq!(
            check("Cmd+Ctrl+V", Action::PushToTalk).unwrap_err(),
            "This shortcut is already in use"
        );
    }

    #[test]
    fn warns_when_hands_free_sits_inside_push_to_talk() {
        let binding = |action, text: &str| Binding::new(action, text.parse::<Trigger>().unwrap());
        assert_eq!(overlap_warning(&macos_defaults(true)), None);
        let overlapping = [
            binding(Action::PushToTalk, "Ctrl+Opt+Space"),
            binding(Action::HandsFree, "Ctrl+Opt"),
        ];
        assert!(
            overlap_warning(&overlapping)
                .is_some_and(|warning| warning.starts_with("Warning: Your Hands-free"))
        );
    }
}
