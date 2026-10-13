use handy_keys::{Hotkey, Key, Modifiers};
use serde::Serialize;

/// Everything a global shortcut can trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    PushToTalk,
    HandsFree,
    /// Only bound while a dictation is recording, so Esc reaches apps otherwise.
    Cancel,
    PasteLast,
    CopyLast,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::PushToTalk => "Push to talk",
            Self::HandsFree => "Hands-free mode",
            Self::Cancel => "Cancel dictation",
            Self::PasteLast => "Paste last transcript",
            Self::CopyLast => "Copy last transcript",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Binding {
    pub action: Action,
    pub hotkey: Hotkey,
}

impl Binding {
    fn new(action: Action, modifiers: Modifiers, key: Option<Key>) -> Self {
        Self {
            action,
            hotkey: Hotkey { modifiers, key },
        }
    }
}

/// The macOS defaults from Wispr Flow: Fn to talk, Fn+Space hands-free,
/// Ctrl+Cmd+V / Ctrl+Cmd+C for the last transcript.
pub fn macos_defaults() -> Vec<Binding> {
    vec![
        Binding::new(Action::PushToTalk, Modifiers::FN, None),
        Binding::new(Action::HandsFree, Modifiers::FN, Some(Key::Space)),
        Binding::new(Action::Cancel, Modifiers::empty(), Some(Key::Escape)),
        Binding::new(
            Action::PasteLast,
            Modifiers::CMD | Modifiers::CTRL,
            Some(Key::V),
        ),
        Binding::new(
            Action::CopyLast,
            Modifiers::CMD | Modifiers::CTRL,
            Some(Key::C),
        ),
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
        label.insert_str(0, if hotkey.key.is_some() { "fn " } else { "fn" });
    }
    match hotkey.key {
        Some(Key::Escape) => label.push_str("Esc"),
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
        ] {
            assert_eq!(defaults.iter().filter(|b| b.action == action).count(), 1);
        }
    }
}
