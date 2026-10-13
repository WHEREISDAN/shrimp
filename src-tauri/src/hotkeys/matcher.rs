use std::collections::HashMap;
use std::time::{Duration, Instant};

use handy_keys::{Key, KeyEvent, Modifiers};

use super::trigger::{MouseButton, Trigger};
use super::{Action, Binding};

/// Press or release of a bound shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutEvent {
    pub action: Action,
    pub pressed: bool,
    /// Modifiers held beyond the binding's, for pass-through bindings.
    pub extra: Modifiers,
}

impl ShortcutEvent {
    pub fn new(action: Action, pressed: bool) -> Self {
        Self {
            action,
            pressed,
            extra: Modifiers::empty(),
        }
    }
}

/// What the matcher makes of one input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matched {
    Shortcut(ShortcutEvent),
    /// A key that is no shortcut went down while a held shortcut is down:
    /// another app's shortcut (CORE-12).
    OtherKey,
}

/// A mouse button event from the mouse hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    pub button: MouseButton,
    pub down: bool,
    pub modifiers: Modifiers,
}

/// Keys handy-keys reports that the matcher leaves alone: mouse buttons come
/// from Shrimp's own mouse hook.
const MOUSE_KEYS: [Key; 5] = [
    Key::MouseLeft,
    Key::MouseRight,
    Key::MouseMiddle,
    Key::MouseX1,
    Key::MouseX2,
];

/// Turns raw key and mouse events into shortcut presses and releases. A
/// shortcut is pressed when its key or button goes down with exactly its
/// modifiers, and released when that key or button goes up or, for
/// modifier-only shortcuts such as Fn, when the held modifiers stop matching.
/// Caps Lock reports its lock state, not the key, so its release comes from
/// [`Matcher::release_key`].
#[derive(Debug, Default)]
pub struct Matcher {
    bindings: Vec<Binding>,
    pressed: HashMap<Action, Instant>,
}

impl Matcher {
    pub fn set_bindings(&mut self, bindings: &[Binding]) {
        self.bindings = bindings.to_vec();
        self.pressed
            .retain(|action, _| bindings.iter().any(|binding| binding.action == *action));
    }

    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// Whether a bound Caps Lock is held, so its release must be watched.
    pub fn caps_lock_held(&self) -> bool {
        self.bindings.iter().any(|binding| {
            binding.trigger.key() == Some(Key::CapsLock)
                && self.pressed.contains_key(&binding.action)
        })
    }

    pub fn process(&mut self, event: &KeyEvent, now: Instant) -> Vec<Matched> {
        if event.key.is_some_and(|key| MOUSE_KEYS.contains(&key)) {
            return Vec::new();
        }
        if event.key == Some(Key::CapsLock) && !event.is_key_down {
            return Vec::new();
        }
        if !event.is_key_down {
            return self.release(event);
        }
        let pressed = self.press(
            event.modifiers,
            |trigger| match trigger {
                Trigger::Keys(hotkey) => hotkey.key == event.key,
                Trigger::Mouse { .. } => false,
            },
            now,
        );
        let holding = [Action::PushToTalk, Action::CommandMode]
            .iter()
            .any(|action| self.pressed.contains_key(action));
        if pressed.is_empty() && event.key.is_some_and(|key| key != Key::CapsLock) && holding {
            return vec![Matched::OtherKey];
        }
        pressed
    }

    pub fn process_mouse(&mut self, event: &MouseEvent, now: Instant) -> Vec<Matched> {
        if event.down {
            return self.press(
                event.modifiers,
                |trigger| trigger.mouse() == Some(event.button),
                now,
            );
        }
        self.release_where(|binding| binding.trigger.mouse() == Some(event.button))
    }

    /// The key went up, as seen outside the key events (Caps Lock).
    pub fn release_key(&mut self, key: Key) -> Vec<Matched> {
        self.release_where(|binding| binding.trigger.key() == Some(key))
    }

    /// Releases shortcuts that have looked held for `after` or longer: their
    /// release was missed (CORE-14). `protected` holds a dictation that is
    /// running, which a long hold must not cut short.
    pub fn release_stuck(
        &mut self,
        now: Instant,
        after: Duration,
        protected: &[Action],
    ) -> Vec<Matched> {
        let stuck: Vec<Action> = self
            .pressed
            .iter()
            .filter(|(action, since)| {
                !protected.contains(action) && now.duration_since(**since) >= after
            })
            .map(|(action, _)| *action)
            .collect();
        stuck
            .into_iter()
            .map(|action| {
                self.pressed.remove(&action);
                log::info!("{action:?} looked stuck; released it");
                Matched::Shortcut(ShortcutEvent::new(action, false))
            })
            .collect()
    }

    fn press(
        &mut self,
        modifiers: Modifiers,
        input: impl Fn(&Trigger) -> bool,
        now: Instant,
    ) -> Vec<Matched> {
        let matched: Vec<(Action, Modifiers)> = self
            .bindings
            .iter()
            .filter(|binding| {
                input(&binding.trigger) && !self.pressed.contains_key(&binding.action)
            })
            .filter_map(|binding| {
                binding
                    .extra_modifiers(modifiers)
                    .map(|extra| (binding.action, extra))
            })
            .collect();
        matched
            .into_iter()
            .filter_map(|(action, extra)| {
                // Several bindings of one action can match the same input.
                self.pressed
                    .insert(action, now)
                    .is_none()
                    .then_some(Matched::Shortcut(ShortcutEvent {
                        extra,
                        ..ShortcutEvent::new(action, true)
                    }))
            })
            .collect()
    }

    fn release(&mut self, event: &KeyEvent) -> Vec<Matched> {
        self.release_where(|binding| match (binding.trigger, event.key) {
            (Trigger::Keys(hotkey), Some(key)) => hotkey.key == Some(key),
            (Trigger::Keys(hotkey), None) => {
                hotkey.key.is_none() && !hotkey.modifiers.matches(event.modifiers)
            }
            (Trigger::Mouse { .. }, _) => false,
        })
    }

    fn release_where(&mut self, released: impl Fn(&Binding) -> bool) -> Vec<Matched> {
        let actions: Vec<Action> = self
            .bindings
            .iter()
            .filter(|binding| self.pressed.contains_key(&binding.action) && released(binding))
            .map(|binding| binding.action)
            .collect();
        actions
            .into_iter()
            .filter_map(|action| {
                self.pressed
                    .remove(&action)
                    .map(|_| Matched::Shortcut(ShortcutEvent::new(action, false)))
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
        matcher.set_bindings(&macos_defaults(true));
        matcher
    }

    fn event(action: Action, pressed: bool) -> Matched {
        Matched::Shortcut(ShortcutEvent::new(action, pressed))
    }

    #[test]
    fn fn_alone_presses_and_releases_push_to_talk() {
        let mut matcher = matcher();
        assert_eq!(
            matcher.process(&flags(Modifiers::FN, true), Instant::now()),
            vec![event(Action::PushToTalk, true)]
        );
        assert_eq!(
            matcher.process(&flags(Modifiers::empty(), false), Instant::now()),
            vec![event(Action::PushToTalk, false)]
        );
    }

    #[test]
    fn fn_space_presses_hands_free_while_push_to_talk_stays_held() {
        let mut matcher = matcher();
        matcher.process(&flags(Modifiers::FN, true), Instant::now());
        assert_eq!(
            matcher.process(&key(Modifiers::FN, Key::Space, true), Instant::now()),
            vec![event(Action::HandsFree, true)]
        );
        assert_eq!(
            matcher.process(&key(Modifiers::FN, Key::Space, false), Instant::now()),
            vec![event(Action::HandsFree, false)]
        );
        assert_eq!(
            matcher.process(&flags(Modifiers::empty(), false), Instant::now()),
            vec![event(Action::PushToTalk, false)]
        );
    }

    #[test]
    fn plain_space_and_fn_with_other_modifiers_do_not_match() {
        let mut matcher = matcher();
        assert!(
            matcher
                .process(&key(Modifiers::empty(), Key::Space, true), Instant::now())
                .is_empty()
        );
        assert!(
            matcher
                .process(
                    &flags(Modifiers::FN | Modifiers::SHIFT_LEFT, true),
                    Instant::now()
                )
                .is_empty()
        );
    }

    #[test]
    fn unbound_actions_are_ignored() {
        let mut matcher = Matcher::default();
        let bindings: Vec<Binding> = macos_defaults(true)
            .into_iter()
            .filter(|binding| binding.action != Action::Cancel)
            .collect();
        matcher.set_bindings(&bindings);
        assert!(
            matcher
                .process(&key(Modifiers::empty(), Key::Escape, true), Instant::now())
                .is_empty()
        );
    }

    #[test]
    fn fn_then_ctrl_presses_command_mode_while_push_to_talk_stays_held() {
        let mut matcher = matcher();
        assert_eq!(
            matcher.process(&flags(Modifiers::FN, true), Instant::now()),
            vec![event(Action::PushToTalk, true)]
        );
        assert_eq!(
            matcher.process(
                &flags(Modifiers::FN | Modifiers::CTRL_LEFT, true),
                Instant::now()
            ),
            vec![event(Action::CommandMode, true)]
        );
        assert_eq!(
            matcher.process(&flags(Modifiers::FN, false), Instant::now()),
            vec![event(Action::CommandMode, false)]
        );
        assert_eq!(
            matcher.process(&flags(Modifiers::empty(), false), Instant::now()),
            vec![event(Action::PushToTalk, false)]
        );
    }

    #[test]
    fn ctrl_then_fn_presses_only_command_mode() {
        let mut matcher = matcher();
        assert!(
            matcher
                .process(&flags(Modifiers::CTRL_LEFT, true), Instant::now())
                .is_empty()
        );
        assert_eq!(
            matcher.process(
                &flags(Modifiers::FN | Modifiers::CTRL_LEFT, true),
                Instant::now()
            ),
            vec![event(Action::CommandMode, true)]
        );
    }

    #[test]
    fn press_enter_passes_extra_modifiers_through() {
        let mut matcher = Matcher::default();
        let binding = Binding::with_passthrough(
            Action::PressEnter,
            "Opt+Return".parse::<handy_keys::Hotkey>().unwrap(),
        );
        matcher.set_bindings(&[binding]);
        let pressed = matcher.process(
            &key(
                Modifiers::OPT_LEFT | Modifiers::SHIFT_LEFT,
                Key::Return,
                true,
            ),
            Instant::now(),
        );
        let [Matched::Shortcut(pressed)] = pressed.as_slice() else {
            panic!("expected one press, got {pressed:?}");
        };
        assert_eq!(pressed.action, Action::PressEnter);
        assert_eq!(pressed.extra, Modifiers::SHIFT);
    }

    #[test]
    fn ctrl_cmd_v_presses_paste_last() {
        let mut matcher = matcher();
        assert_eq!(
            matcher.process(
                &key(Modifiers::CMD_LEFT | Modifiers::CTRL_LEFT, Key::V, true),
                Instant::now()
            ),
            vec![event(Action::PasteLast, true)]
        );
    }

    fn mouse(button: u8, down: bool, modifiers: Modifiers) -> MouseEvent {
        MouseEvent {
            button: MouseButton::new(i64::from(button)).unwrap(),
            down,
            modifiers,
        }
    }

    #[test]
    fn mouse_buttons_press_and_release_their_action() {
        let mut matcher = Matcher::default();
        matcher.set_bindings(&[
            Binding::new(Action::PushToTalk, "Mouse4".parse::<Trigger>().unwrap()),
            Binding::new(
                Action::HandsFree,
                "Ctrl+MouseMiddle".parse::<Trigger>().unwrap(),
            ),
        ]);
        let now = Instant::now();
        assert_eq!(
            matcher.process_mouse(&mouse(3, true, Modifiers::empty()), now),
            vec![event(Action::PushToTalk, true)]
        );
        assert_eq!(
            matcher.process_mouse(&mouse(3, false, Modifiers::empty()), now),
            vec![event(Action::PushToTalk, false)]
        );
        assert!(
            matcher
                .process_mouse(&mouse(2, true, Modifiers::empty()), now)
                .is_empty(),
            "the middle button needs Ctrl"
        );
        assert_eq!(
            matcher.process_mouse(&mouse(2, true, Modifiers::CTRL_LEFT), now),
            vec![event(Action::HandsFree, true)]
        );
    }

    #[test]
    fn handy_keys_mouse_reports_are_ignored() {
        let mut matcher = Matcher::default();
        matcher.set_bindings(&[Binding::new(
            Action::PushToTalk,
            "MouseMiddle".parse::<Trigger>().unwrap(),
        )]);
        assert!(
            matcher
                .process(
                    &key(Modifiers::empty(), Key::MouseMiddle, true),
                    Instant::now()
                )
                .is_empty()
        );
    }

    #[test]
    fn another_key_while_holding_push_to_talk_is_reported() {
        let mut matcher = matcher();
        let now = Instant::now();
        matcher.process(&flags(Modifiers::FN, true), now);
        assert_eq!(
            matcher.process(&key(Modifiers::FN, Key::F, true), now),
            vec![Matched::OtherKey]
        );
        assert!(
            matcher
                .process(&key(Modifiers::empty(), Key::F, true), now)
                .iter()
                .all(|matched| *matched == Matched::OtherKey)
        );
        let mut idle = matcher_without_holds();
        assert!(
            idle.process(&key(Modifiers::empty(), Key::F, true), now)
                .is_empty()
        );
    }

    fn matcher_without_holds() -> Matcher {
        let mut matcher = Matcher::default();
        matcher.set_bindings(&macos_defaults(true));
        matcher
    }

    #[test]
    fn stuck_shortcuts_are_released_after_the_limit_unless_protected() {
        let mut matcher = matcher();
        let start = Instant::now();
        let paste = key(Modifiers::CMD_LEFT | Modifiers::CTRL_LEFT, Key::V, true);
        assert_eq!(
            matcher.process(&paste, start),
            vec![event(Action::PasteLast, true)]
        );
        assert!(
            matcher.process(&paste, start).is_empty(),
            "still looks held"
        );
        matcher.process(&flags(Modifiers::FN, true), start);
        let limit = Duration::from_secs(10);
        assert!(
            matcher
                .release_stuck(start + Duration::from_secs(9), limit, &[])
                .is_empty()
        );
        let released = matcher.release_stuck(
            start + Duration::from_secs(10),
            limit,
            &[Action::PushToTalk],
        );
        assert_eq!(released, vec![event(Action::PasteLast, false)]);
        assert_eq!(
            matcher.process(&paste, start + Duration::from_secs(11)),
            vec![event(Action::PasteLast, true)],
            "works again"
        );
        assert!(
            matcher
                .release_stuck(
                    start + Duration::from_secs(60),
                    limit,
                    &[Action::PushToTalk, Action::PasteLast]
                )
                .is_empty(),
            "a running dictation is never cut short"
        );
    }

    #[test]
    fn caps_lock_releases_only_when_told() {
        let mut matcher = Matcher::default();
        matcher.set_bindings(&[Binding::new(
            Action::PushToTalk,
            "CapsLock".parse::<Trigger>().unwrap(),
        )]);
        let now = Instant::now();
        assert_eq!(
            matcher.process(&key(Modifiers::empty(), Key::CapsLock, true), now),
            vec![event(Action::PushToTalk, true)]
        );
        assert!(matcher.caps_lock_held());
        assert!(
            matcher
                .process(&key(Modifiers::empty(), Key::CapsLock, false), now)
                .is_empty(),
            "the lock turning off is not the key going up"
        );
        assert_eq!(
            matcher.release_key(Key::CapsLock),
            vec![event(Action::PushToTalk, false)]
        );
    }
}
