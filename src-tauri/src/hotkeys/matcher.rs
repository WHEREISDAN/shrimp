use std::collections::HashSet;

use handy_keys::KeyEvent;

use super::{Action, Binding};

/// Press or release of a bound shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutEvent {
    pub action: Action,
    pub pressed: bool,
}

/// Turns raw key events into shortcut presses and releases. A shortcut is
/// pressed when its key goes down with exactly its modifiers, and released
/// when that key goes up or, for modifier-only shortcuts such as Fn, when the
/// held modifiers stop matching.
#[derive(Debug, Default)]
pub struct Matcher {
    bindings: Vec<Binding>,
    pressed: HashSet<Action>,
}

impl Matcher {
    pub fn set_bindings(&mut self, bindings: &[Binding]) {
        self.bindings = bindings.to_vec();
        let bound: HashSet<Action> = bindings.iter().map(|binding| binding.action).collect();
        self.pressed.retain(|action| bound.contains(action));
    }

    pub fn process(&mut self, event: &KeyEvent) -> Vec<ShortcutEvent> {
        if event.is_key_down {
            self.press(event)
        } else {
            self.release(event)
        }
    }

    fn press(&mut self, event: &KeyEvent) -> Vec<ShortcutEvent> {
        let actions: Vec<Action> = self
            .bindings
            .iter()
            .filter(|binding| {
                binding.hotkey.key == event.key
                    && binding.hotkey.modifiers.matches(event.modifiers)
                    && !self.pressed.contains(&binding.action)
            })
            .map(|binding| binding.action)
            .collect();
        actions
            .into_iter()
            .map(|action| {
                self.pressed.insert(action);
                ShortcutEvent {
                    action,
                    pressed: true,
                }
            })
            .collect()
    }

    fn release(&mut self, event: &KeyEvent) -> Vec<ShortcutEvent> {
        let actions: Vec<Action> = self
            .bindings
            .iter()
            .filter(|binding| self.pressed.contains(&binding.action))
            .filter(|binding| match event.key {
                Some(key) => binding.hotkey.key == Some(key),
                None => !binding.hotkey.modifiers.matches(event.modifiers),
            })
            .map(|binding| binding.action)
            .collect();
        actions
            .into_iter()
            .map(|action| {
                self.pressed.remove(&action);
                ShortcutEvent {
                    action,
                    pressed: false,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use handy_keys::{Key, Modifiers};

    use super::*;
    use crate::hotkeys::macos_defaults;

    fn flags(modifiers: Modifiers, down: bool) -> KeyEvent {
        KeyEvent {
            modifiers,
            key: None,
            is_key_down: down,
            changed_modifier: None,
        }
    }

    fn key(modifiers: Modifiers, key: Key, down: bool) -> KeyEvent {
        KeyEvent {
            modifiers,
            key: Some(key),
            is_key_down: down,
            changed_modifier: None,
        }
    }

    fn matcher() -> Matcher {
        let mut matcher = Matcher::default();
        matcher.set_bindings(&macos_defaults());
        matcher
    }

    fn event(action: Action, pressed: bool) -> ShortcutEvent {
        ShortcutEvent { action, pressed }
    }

    #[test]
    fn fn_alone_presses_and_releases_push_to_talk() {
        let mut matcher = matcher();
        assert_eq!(
            matcher.process(&flags(Modifiers::FN, true)),
            vec![event(Action::PushToTalk, true)]
        );
        assert_eq!(
            matcher.process(&flags(Modifiers::empty(), false)),
            vec![event(Action::PushToTalk, false)]
        );
    }

    #[test]
    fn fn_space_presses_hands_free_while_push_to_talk_stays_held() {
        let mut matcher = matcher();
        matcher.process(&flags(Modifiers::FN, true));
        assert_eq!(
            matcher.process(&key(Modifiers::FN, Key::Space, true)),
            vec![event(Action::HandsFree, true)]
        );
        assert_eq!(
            matcher.process(&key(Modifiers::FN, Key::Space, false)),
            vec![event(Action::HandsFree, false)]
        );
        assert_eq!(
            matcher.process(&flags(Modifiers::empty(), false)),
            vec![event(Action::PushToTalk, false)]
        );
    }

    #[test]
    fn plain_space_and_fn_with_other_modifiers_do_not_match() {
        let mut matcher = matcher();
        assert!(
            matcher
                .process(&key(Modifiers::empty(), Key::Space, true))
                .is_empty()
        );
        assert!(
            matcher
                .process(&flags(Modifiers::FN | Modifiers::SHIFT_LEFT, true))
                .is_empty()
        );
    }

    #[test]
    fn unbound_actions_are_ignored() {
        let mut matcher = Matcher::default();
        let bindings: Vec<Binding> = macos_defaults()
            .into_iter()
            .filter(|binding| binding.action != Action::Cancel)
            .collect();
        matcher.set_bindings(&bindings);
        assert!(
            matcher
                .process(&key(Modifiers::empty(), Key::Escape, true))
                .is_empty()
        );
    }

    #[test]
    fn ctrl_cmd_v_presses_paste_last() {
        let mut matcher = matcher();
        assert_eq!(
            matcher.process(&key(
                Modifiers::CMD_LEFT | Modifiers::CTRL_LEFT,
                Key::V,
                true
            )),
            vec![event(Action::PasteLast, true)]
        );
    }
}
