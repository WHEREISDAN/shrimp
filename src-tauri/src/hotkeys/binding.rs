use std::collections::HashSet;

use handy_keys::{Hotkey, Key, Modifiers};
use serde::{Deserialize, Serialize};

/// Everything a global shortcut can trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    PushToTalk,
    HandsFree,
    /// Only bound while a dictation is recording, so Esc reaches apps otherwise.
    Cancel,
    PasteLast,
    CopyLast,
    /// A spoken instruction for the selection, or a question (CMD-01).
    CommandMode,
    /// Sends Enter to the focused app; unbound by default (CORE-15).
    PressEnter,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::PushToTalk => "Push to talk",
            Self::HandsFree => "Hands-free mode",
            Self::Cancel => "Cancel dictation",
            Self::PasteLast => "Paste last transcript",
            Self::CopyLast => "Copy last transcript",
            Self::CommandMode => "Command Mode",
            Self::PressEnter => "Press Enter",
        }
    }
}

/// Modifier groups (either side) that can pass through a binding.
const PASSTHROUGH_GROUPS: [Modifiers; 4] = [
    Modifiers::SHIFT,
    Modifiers::CTRL,
    Modifiers::OPT,
    Modifiers::CMD,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Binding {
    pub action: Action,
    pub hotkey: Hotkey,
    /// Also matches with extra Shift, Control, Option or Command held, which
    /// pass through to the app (Shift+Enter from Press Enter).
    pub passthrough: bool,
}

impl Binding {
    fn of(action: Action, modifiers: Modifiers, key: Option<Key>) -> Self {
        Self::new(action, Hotkey { modifiers, key })
    }

    pub fn new(action: Action, hotkey: Hotkey) -> Self {
        Self {
            action,
            hotkey,
            passthrough: false,
        }
    }

    pub fn with_passthrough(action: Action, hotkey: Hotkey) -> Self {
        Self {
            passthrough: true,
            ..Self::new(action, hotkey)
        }
    }

    /// The modifiers held beyond the binding's own, when `modifiers` match it
    /// (always empty for bindings without pass-through).
    pub fn extra_modifiers(&self, modifiers: Modifiers) -> Option<Modifiers> {
        if self.hotkey.modifiers.matches(modifiers) {
            return Some(Modifiers::empty());
        }
        if !self.passthrough
            || modifiers.contains(Modifiers::FN) != self.hotkey.modifiers.contains(Modifiers::FN)
        {
            return None;
        }
        let mut extra = Modifiers::empty();
        for group in PASSTHROUGH_GROUPS {
            match (
                self.hotkey.modifiers.intersects(group),
                modifiers.intersects(group),
            ) {
                (true, false) => return None,
                (false, true) => extra |= group,
                _ => {}
            }
        }
        Some(extra)
    }

    /// Hotkeys the keyboard hook swallows for this binding: with pass-through,
    /// every combination of extra modifiers too.
    fn blocked(&self) -> Vec<Hotkey> {
        if !self.passthrough {
            return vec![self.hotkey];
        }
        let free: Vec<Modifiers> = PASSTHROUGH_GROUPS
            .into_iter()
            .filter(|group| !self.hotkey.modifiers.intersects(*group))
            .collect();
        (0..1u32 << free.len())
            .map(|mask| {
                let extra = free
                    .iter()
                    .enumerate()
                    .filter(|(bit, _)| mask & (1 << bit) != 0)
                    .fold(Modifiers::empty(), |acc, (_, group)| acc | *group);
                Hotkey {
                    modifiers: self.hotkey.modifiers | extra,
                    key: self.hotkey.key,
                }
            })
            .collect()
    }
}

/// Everything the keyboard hook must swallow for `bindings`.
pub fn blocked_hotkeys(bindings: &[Binding]) -> HashSet<Hotkey> {
    bindings.iter().flat_map(Binding::blocked).collect()
}

/// Backspace cancels Command Mode as well as Esc (CMD-04).
pub fn backspace_cancel() -> Binding {
    Binding::of(Action::Cancel, Modifiers::empty(), Some(Key::Delete))
}

/// The macOS defaults from Wispr Flow: Fn to talk, Fn+Space hands-free,
/// Ctrl+Cmd+V / Ctrl+Cmd+C for the last transcript, Fn+Ctrl for Command Mode.
pub fn macos_defaults() -> Vec<Binding> {
    vec![
        Binding::of(Action::PushToTalk, Modifiers::FN, None),
        Binding::of(Action::HandsFree, Modifiers::FN, Some(Key::Space)),
        Binding::of(Action::Cancel, Modifiers::empty(), Some(Key::Escape)),
        Binding::of(
            Action::PasteLast,
            Modifiers::CMD | Modifiers::CTRL,
            Some(Key::V),
        ),
        Binding::of(
            Action::CopyLast,
            Modifiers::CMD | Modifiers::CTRL,
            Some(Key::C),
        ),
        Binding::of(Action::CommandMode, Modifiers::FN | Modifiers::CTRL, None),
    ]
}

/// Human-readable shortcut in macOS notation, e.g. "fn", "fn Space", "⌃⌘V".
pub fn macos_label(hotkey: &Hotkey) -> String {
    const SYMBOLS: [(Modifiers, &str); 4] = [
        (Modifiers::CTRL, "⌃"),
        (Modifiers::OPT, "⌥"),
        (Modifiers::SHIFT, "⇧"),
        (Modifiers::CMD, "⌘"),
    ];
    let mut label: String = SYMBOLS
        .iter()
        .filter(|(modifier, _)| hotkey.modifiers.intersects(*modifier))
        .map(|(_, symbol)| *symbol)
        .collect();
    if hotkey.modifiers.contains(Modifiers::FN) {
        let spaced = hotkey.key.is_some() || !label.is_empty();
        label.insert_str(0, if spaced { "fn " } else { "fn" });
    }
    match hotkey.key {
        Some(Key::Escape) => label.push_str("Esc"),
        Some(Key::Return) => label.push('↩'),
        Some(Key::Delete) => label.push('⌫'),
        Some(key) => label.push_str(&key.to_string()),
        None => {}
    }
    label
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label_of(action: Action) -> String {
        let binding = macos_defaults()
            .into_iter()
            .find(|binding| binding.action == action)
            .unwrap();
        macos_label(&binding.hotkey)
    }

    #[test]
    fn labels_the_macos_defaults() {
        assert_eq!(label_of(Action::PushToTalk), "fn");
        assert_eq!(label_of(Action::HandsFree), "fn Space");
        assert_eq!(label_of(Action::Cancel), "Esc");
        assert_eq!(label_of(Action::PasteLast), "⌃⌘V");
        assert_eq!(label_of(Action::CopyLast), "⌃⌘C");
        assert_eq!(label_of(Action::CommandMode), "fn ⌃");
    }

    #[test]
    fn passthrough_bindings_match_with_extra_modifiers() {
        let hotkey: Hotkey = "Opt+Return".parse().unwrap();
        let plain = Binding::new(Action::PressEnter, hotkey);
        let through = Binding::with_passthrough(Action::PressEnter, hotkey);
        let shift_opt = Modifiers::OPT_LEFT | Modifiers::SHIFT_LEFT;
        assert_eq!(
            plain.extra_modifiers(Modifiers::OPT_LEFT),
            Some(Modifiers::empty())
        );
        assert_eq!(plain.extra_modifiers(shift_opt), None);
        assert_eq!(through.extra_modifiers(shift_opt), Some(Modifiers::SHIFT));
        assert_eq!(
            through.extra_modifiers(Modifiers::SHIFT_LEFT),
            None,
            "Option is required"
        );
        assert_eq!(
            through.extra_modifiers(Modifiers::OPT_LEFT | Modifiers::FN),
            None,
            "Fn never passes through"
        );
        let blocked = blocked_hotkeys(&[through]);
        assert_eq!(blocked.len(), 8);
        assert!(blocked.contains(&Hotkey {
            modifiers: Modifiers::OPT | Modifiers::SHIFT,
            key: Some(Key::Return)
        }));
    }

    #[test]
    fn every_action_has_exactly_one_default() {
        let defaults = macos_defaults();
        for action in [
            Action::PushToTalk,
            Action::HandsFree,
            Action::Cancel,
            Action::PasteLast,
            Action::CopyLast,
            Action::CommandMode,
        ] {
            assert_eq!(defaults.iter().filter(|b| b.action == action).count(), 1);
        }
    }
}
