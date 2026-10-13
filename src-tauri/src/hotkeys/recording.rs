//! Records a new shortcut from raw key and mouse events (KEYS-01, KEYS-06).
//! Keys save when they are all released and mouse buttons when pressed. Esc
//! alone cancels, unless the shortcut is for Cancel; Ctrl then Esc records
//! Ctrl+Esc. A left or right click cancels.

use handy_keys::{Key, KeyEvent, Modifiers};

use super::matcher::MouseEvent;
use super::trigger::Trigger;

/// What the hooks report while a shortcut is being recorded.
#[derive(Debug, Clone, Copy)]
pub enum RecordInput {
    Key(KeyEvent),
    Mouse(MouseEvent),
    /// A left or right click anywhere.
    Click,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordUpdate {
    /// What is held so far.
    Preview(Trigger),
    Done(Recorded),
    Cancelled,
}

/// A recorded shortcut, before validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recorded {
    pub trigger: Trigger,
    /// Both the left and the right key of one modifier were held (KEYS-03).
    pub both_sides: bool,
}

const SIDES: [(Modifiers, Modifiers); 4] = [
    (Modifiers::CTRL_LEFT, Modifiers::CTRL_RIGHT),
    (Modifiers::OPT_LEFT, Modifiers::OPT_RIGHT),
    (Modifiers::SHIFT_LEFT, Modifiers::SHIFT_RIGHT),
    (Modifiers::CMD_LEFT, Modifiers::CMD_RIGHT),
];

const GROUPS: [Modifiers; 4] = [
    Modifiers::CTRL,
    Modifiers::OPT,
    Modifiers::SHIFT,
    Modifiers::CMD,
];

const IGNORED_KEYS: [Key; 5] = [
    Key::MouseLeft,
    Key::MouseRight,
    Key::MouseMiddle,
    Key::MouseX1,
    Key::MouseX2,
];

#[derive(Debug)]
pub struct Recording {
    /// Esc alone records instead of cancelling.
    for_cancel: bool,
    modifiers: Modifiers,
    key: Option<Key>,
    candidate: Option<Trigger>,
    both_sides: bool,
}

/// Shortcuts match either side of a modifier, so sides are dropped.
fn either_side(modifiers: Modifiers) -> Modifiers {
    GROUPS
        .iter()
        .filter(|group| modifiers.intersects(**group))
        .fold(modifiers & Modifiers::FN, |acc, group| acc | *group)
}

fn has_both_sides(modifiers: Modifiers) -> bool {
    SIDES
        .iter()
        .any(|(left, right)| modifiers.contains(*left) && modifiers.contains(*right))
}

impl Recording {
    pub fn new(for_cancel: bool) -> Self {
        Self {
            for_cancel,
            modifiers: Modifiers::empty(),
            key: None,
            candidate: None,
            both_sides: false,
        }
    }

    pub fn handle(&mut self, input: RecordInput) -> Option<RecordUpdate> {
        match input {
            RecordInput::Click => Some(RecordUpdate::Cancelled),
            RecordInput::Mouse(event) if event.down => Some(self.done(Trigger::Mouse {
                modifiers: either_side(self.modifiers),
                button: event.button,
            })),
            RecordInput::Mouse(_) => None,
            RecordInput::Key(event) => self.key_event(&event),
        }
    }

    fn key_event(&mut self, event: &KeyEvent) -> Option<RecordUpdate> {
        if event.key.is_some_and(|key| IGNORED_KEYS.contains(&key)) {
            return None;
        }
        self.modifiers = event.modifiers;
        self.both_sides |= has_both_sides(event.modifiers);
        match (event.key, event.is_key_down) {
            (Some(Key::Escape), true)
                if !self.for_cancel && event.modifiers.is_empty() && self.candidate.is_none() =>
            {
                Some(RecordUpdate::Cancelled)
            }
            // Caps Lock reports its lock state, never its release.
            (Some(Key::CapsLock), true) => Some(self.done(Trigger::keys(
                either_side(event.modifiers),
                Some(Key::CapsLock),
            ))),
            (Some(Key::CapsLock), false) => None,
            (Some(key), true) => {
                self.key = Some(key);
                Some(self.update())
            }
            (Some(_), false) => {
                self.key = None;
                self.finish_if_released()
            }
            (None, true) => Some(self.update()),
            (None, false) => self.finish_if_released(),
        }
    }

    fn update(&mut self) -> RecordUpdate {
        let trigger = Trigger::keys(either_side(self.modifiers), self.key);
        self.candidate = Some(trigger);
        RecordUpdate::Preview(trigger)
    }

    fn finish_if_released(&mut self) -> Option<RecordUpdate> {
        if !self.modifiers.is_empty() || self.key.is_some() {
            return None;
        }
        self.candidate.map(|trigger| self.done(trigger))
    }

    fn done(&self, trigger: Trigger) -> RecordUpdate {
        RecordUpdate::Done(Recorded {
            trigger,
            both_sides: self.both_sides || has_both_sides(self.modifiers),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkeys::MouseButton;

    fn flags(modifiers: Modifiers, down: bool) -> RecordInput {
        RecordInput::Key(KeyEvent {
            modifiers,
            key: None,
            is_key_down: down,
            changed_modifier: None,
        })
    }

    fn key(modifiers: Modifiers, key: Key, down: bool) -> RecordInput {
        RecordInput::Key(KeyEvent {
            modifiers,
            key: Some(key),
            is_key_down: down,
            changed_modifier: None,
        })
    }

    fn run(recording: &mut Recording, inputs: &[RecordInput]) -> Vec<RecordUpdate> {
        inputs
            .iter()
            .filter_map(|input| recording.handle(*input))
            .collect()
    }

    fn done(text: &str) -> RecordUpdate {
        RecordUpdate::Done(Recorded {
            trigger: text.parse().unwrap(),
            both_sides: false,
        })
    }

    #[test]
    fn keys_save_when_everything_is_released() {
        let mut recording = Recording::new(false);
        let ctrl = Modifiers::CTRL_LEFT;
        let both = ctrl | Modifiers::OPT_LEFT;
        let updates = run(
            &mut recording,
            &[
                flags(ctrl, true),
                flags(both, true),
                key(both, Key::Space, true),
                key(both, Key::Space, false),
                flags(ctrl, false),
            ],
        );
        assert!(matches!(updates.last(), Some(RecordUpdate::Preview(_))));
        assert_eq!(
            recording.handle(flags(Modifiers::empty(), false)),
            Some(done("Ctrl+Opt+Space"))
        );
    }

    #[test]
    fn modifier_only_shortcuts_record_like_fn() {
        let mut recording = Recording::new(false);
        let updates = run(
            &mut recording,
            &[flags(Modifiers::FN, true), flags(Modifiers::empty(), false)],
        );
        assert_eq!(updates.last(), Some(&done("Fn")));
    }

    #[test]
    fn mouse_buttons_save_on_press_with_held_modifiers() {
        let mut recording = Recording::new(false);
        recording.handle(flags(Modifiers::SHIFT_RIGHT, true));
        let update = recording.handle(RecordInput::Mouse(MouseEvent {
            button: MouseButton::new(3).unwrap(),
            down: true,
            modifiers: Modifiers::SHIFT_LEFT,
        }));
        assert_eq!(update, Some(done("Shift+Mouse4")));
    }

    #[test]
    fn esc_alone_or_a_click_cancels() {
        let mut recording = Recording::new(false);
        assert_eq!(
            recording.handle(key(Modifiers::empty(), Key::Escape, true)),
            Some(RecordUpdate::Cancelled)
        );
        let mut recording = Recording::new(false);
        assert_eq!(
            recording.handle(RecordInput::Click),
            Some(RecordUpdate::Cancelled)
        );
    }

    #[test]
    fn ctrl_then_esc_records_ctrl_esc_and_cancel_records_esc() {
        let mut recording = Recording::new(false);
        let updates = run(
            &mut recording,
            &[
                flags(Modifiers::CTRL_LEFT, true),
                key(Modifiers::CTRL_LEFT, Key::Escape, true),
                key(Modifiers::CTRL_LEFT, Key::Escape, false),
                flags(Modifiers::empty(), false),
            ],
        );
        assert_eq!(updates.last(), Some(&done("Ctrl+Escape")));

        let mut recording = Recording::new(true);
        let updates = run(
            &mut recording,
            &[
                key(Modifiers::empty(), Key::Escape, true),
                key(Modifiers::empty(), Key::Escape, false),
            ],
        );
        assert_eq!(updates.last(), Some(&done("Escape")));
    }

    #[test]
    fn caps_lock_saves_on_press() {
        let mut recording = Recording::new(false);
        assert_eq!(
            recording.handle(key(Modifiers::empty(), Key::CapsLock, true)),
            Some(done("CapsLock"))
        );
    }

    #[test]
    fn both_sides_of_a_modifier_are_flagged() {
        let mut recording = Recording::new(false);
        let both = Modifiers::CMD_LEFT | Modifiers::CMD_RIGHT;
        let updates = run(
            &mut recording,
            &[
                flags(Modifiers::CMD_LEFT, true),
                flags(both, true),
                flags(Modifiers::CMD_RIGHT, false),
                flags(Modifiers::empty(), false),
            ],
        );
        let Some(RecordUpdate::Done(recorded)) = updates.last() else {
            panic!("not done: {updates:?}");
        };
        assert!(recorded.both_sides);
    }
}
