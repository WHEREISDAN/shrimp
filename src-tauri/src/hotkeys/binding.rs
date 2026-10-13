use std::collections::HashSet;

use handy_keys::{Hotkey, Key, Modifiers};
use serde::{Deserialize, Serialize};

use super::trigger::{MouseButton, Trigger};
use crate::t;

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
    /// Rewrites the selection with the Polish rules (TRF-01).
    Polish,
    /// Turns the selection into a structured AI prompt (TRF-02).
    PromptEngineer,
    /// A custom transform, by its position in the settings (TRF-03).
    CustomTransform(u8),
    /// Shows what the last transform changed (TRF-04).
    ViewChanges,
}

impl Action {
    pub fn label(self) -> String {
        match self {
            Self::PushToTalk => t!("name.action.pushToTalk"),
            Self::HandsFree => t!("name.action.handsFree"),
            Self::Cancel => t!("name.action.cancel"),
            Self::PasteLast => t!("name.action.pasteLast"),
            Self::CopyLast => t!("name.action.copyLast"),
            Self::CommandMode => t!("name.action.commandMode"),
            Self::PressEnter => t!("name.action.pressEnter"),
            Self::Polish => t!("name.action.polish"),
            Self::PromptEngineer => t!("name.action.promptEngineer"),
            Self::CustomTransform(_) => t!("name.action.customTransform"),
            Self::ViewChanges => t!("name.action.viewChanges"),
        }
    }
}

impl Action {
    /// The key a binding list is stored under in the settings; custom
    /// transforms are stored by their id instead.
    pub fn storage_key(self) -> Option<&'static str> {
        Some(match self {
            Self::PushToTalk => "pushToTalk",
            Self::HandsFree => "handsFree",
            Self::Cancel => "cancel",
            Self::PasteLast => "pasteLast",
            Self::CopyLast => "copyLast",
            Self::CommandMode => "commandMode",
            Self::PressEnter => "pressEnter",
            Self::Polish => "polish",
            Self::PromptEngineer => "promptEngineer",
            Self::ViewChanges => "viewChanges",
            Self::CustomTransform(_) => return None,
        })
    }

    /// Up to 4 bindings per action; Cancel takes 1 (KEYS-02).
    pub fn max_bindings(self) -> usize {
        if self == Self::Cancel { 1 } else { 4 }
    }

    /// Actions that must keep at least one binding (KEYS-02).
    pub fn required(self) -> bool {
        matches!(
            self,
            Self::PushToTalk | Self::CommandMode | Self::PasteLast | Self::CopyLast | Self::Cancel
        )
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
    pub trigger: Trigger,
    /// Also matches with extra Shift, Control, Option or Command held, which
    /// pass through to the app (Shift+Enter from Press Enter).
    pub passthrough: bool,
}

impl Binding {
    fn of(action: Action, modifiers: Modifiers, key: Option<Key>) -> Self {
        Self::new(action, Trigger::keys(modifiers, key))
    }

    pub fn new(action: Action, trigger: impl Into<Trigger>) -> Self {
        Self {
            action,
            trigger: trigger.into(),
            passthrough: false,
        }
    }

    pub fn with_passthrough(action: Action, trigger: impl Into<Trigger>) -> Self {
        Self {
            passthrough: true,
            ..Self::new(action, trigger)
        }
    }

    /// The modifiers held beyond the binding's own, when `modifiers` match it
    /// (always empty for bindings without pass-through).
    pub fn extra_modifiers(&self, modifiers: Modifiers) -> Option<Modifiers> {
        let own = self.trigger.modifiers();
        if own.matches(modifiers) {
            return Some(Modifiers::empty());
        }
        if !self.passthrough || modifiers.contains(Modifiers::FN) != own.contains(Modifiers::FN) {
            return None;
        }
        let mut extra = Modifiers::empty();
        for group in PASSTHROUGH_GROUPS {
            match (own.intersects(group), modifiers.intersects(group)) {
                (true, false) => return None,
                (false, true) => extra |= group,
                _ => {}
            }
        }
        Some(extra)
    }

    /// Key combinations the keyboard hook swallows for this binding: with
    /// pass-through, every combination of extra modifiers too.
    fn blocked(&self) -> Vec<Hotkey> {
        let Trigger::Keys(hotkey) = self.trigger else {
            return Vec::new();
        };
        if !self.passthrough {
            return vec![hotkey];
        }
        let free: Vec<Modifiers> = PASSTHROUGH_GROUPS
            .into_iter()
            .filter(|group| !hotkey.modifiers.intersects(*group))
            .collect();
        (0..1u32 << free.len())
            .map(|mask| {
                let extra = free
                    .iter()
                    .enumerate()
                    .filter(|(bit, _)| mask & (1 << bit) != 0)
                    .fold(Modifiers::empty(), |acc, (_, group)| acc | *group);
                Hotkey {
                    modifiers: hotkey.modifiers | extra,
                    key: hotkey.key,
                }
            })
            .collect()
    }
}

/// Everything the keyboard hook must swallow for `bindings`.
pub fn blocked_hotkeys(bindings: &[Binding]) -> HashSet<Hotkey> {
    bindings.iter().flat_map(Binding::blocked).collect()
}

/// Mouse buttons used by any binding. They never reach other apps, whatever
/// modifiers are held (CORE-13).
pub fn blocked_buttons(bindings: &[Binding]) -> HashSet<MouseButton> {
    bindings
        .iter()
        .filter_map(|binding| binding.trigger.mouse())
        .collect()
}

/// Backspace cancels Command Mode as well as Esc (CMD-04), also while the
/// command's modifiers are held.
pub fn backspace_cancel() -> Binding {
    Binding::with_passthrough(
        Action::Cancel,
        Trigger::keys(Modifiers::empty(), Some(Key::Delete)),
    )
}

/// The macOS defaults from Wispr Flow: Fn to talk, Fn+Space hands-free,
/// Fn+Ctrl for Command Mode, Ctrl+Cmd+V / Ctrl+Cmd+C for the last
/// transcript. Without an Apple Fn key, Ctrl+Opt takes Fn's place (CORE-10).
pub fn macos_defaults(fn_key: bool) -> Vec<Binding> {
    let (talk, command) = if fn_key {
        (Modifiers::FN, Modifiers::FN | Modifiers::CTRL)
    } else {
        (
            Modifiers::CTRL | Modifiers::OPT,
            Modifiers::CMD | Modifiers::CTRL | Modifiers::OPT,
        )
    };
    vec![
        Binding::of(Action::PushToTalk, talk, None),
        Binding::of(Action::HandsFree, talk, Some(Key::Space)),
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
        Binding::of(Action::CommandMode, command, None),
        Binding::of(Action::Polish, Modifiers::OPT, Some(Key::Num1)),
        Binding::of(Action::PromptEngineer, Modifiers::OPT, Some(Key::Num2)),
        Binding::of(Action::ViewChanges, Modifiers::OPT, Some(Key::O)),
    ]
}

/// The Windows defaults from Wispr Flow (WIN-02): Ctrl+Win to talk,
/// Ctrl+Win+Space hands-free, Ctrl+Win+Alt for Command Mode, Shift+Alt+Z /
/// Shift+Alt+X for the last transcript and Win+Alt for the transforms.
pub fn windows_defaults() -> Vec<Binding> {
    let talk = Modifiers::CTRL | Modifiers::CMD;
    let last = Modifiers::SHIFT | Modifiers::OPT;
    let transforms = Modifiers::CMD | Modifiers::OPT;
    vec![
        Binding::of(Action::PushToTalk, talk, None),
        Binding::of(Action::HandsFree, talk, Some(Key::Space)),
        Binding::of(Action::Cancel, Modifiers::empty(), Some(Key::Escape)),
        Binding::of(Action::PasteLast, last, Some(Key::Z)),
        Binding::of(Action::CopyLast, last, Some(Key::X)),
        Binding::of(Action::CommandMode, talk | Modifiers::OPT, None),
        Binding::of(Action::Polish, transforms, Some(Key::Num1)),
        Binding::of(Action::PromptEngineer, transforms, Some(Key::Num2)),
        Binding::of(Action::ViewChanges, transforms, Some(Key::O)),
    ]
}

/// Human-readable shortcut in macOS notation, e.g. "fn", "fn Space", "⌃⌘V",
/// "⌃ Mouse 4".
pub fn macos_label(trigger: &Trigger) -> String {
    const SYMBOLS: [(Modifiers, &str); 4] = [
        (Modifiers::CTRL, "⌃"),
        (Modifiers::OPT, "⌥"),
        (Modifiers::SHIFT, "⇧"),
        (Modifiers::CMD, "⌘"),
    ];
    let modifiers = trigger.modifiers();
    let mut label: String = SYMBOLS
        .iter()
        .filter(|(modifier, _)| modifiers.intersects(*modifier))
        .map(|(_, symbol)| *symbol)
        .collect();
    let has_input = trigger.key().is_some() || trigger.mouse().is_some();
    if modifiers.contains(Modifiers::FN) {
        let spaced = has_input || !label.is_empty();
        label.insert_str(0, if spaced { "fn " } else { "fn" });
    }
    match trigger {
        Trigger::Keys(hotkey) => match hotkey.key {
            Some(Key::Escape) => label.push_str("Esc"),
            Some(Key::Return) => label.push('↩'),
            Some(Key::Delete) => label.push('⌫'),
            Some(Key::ForwardDelete) => label.push('⌦'),
            Some(Key::CapsLock) => label.push_str(&t!("name.key.capsLock")),
            Some(Key::Space) => label.push_str(&t!("name.key.space")),
            Some(key) => label.push_str(&key.to_string()),
            None => {}
        },
        Trigger::Mouse { button, .. } => {
            if !label.is_empty() && !label.ends_with(' ') {
                label.push(' ');
            }
            label.push_str(&button.label());
        }
    }
    label
}

/// Human-readable shortcut in Windows notation, e.g. "Ctrl+Win",
/// "Shift+Alt+Z", "Ctrl+Mouse 4".
pub fn windows_label(trigger: &Trigger) -> String {
    const NAMES: [(Modifiers, &str); 4] = [
        (Modifiers::CTRL, "Ctrl"),
        (Modifiers::CMD, "Win"),
        (Modifiers::SHIFT, "Shift"),
        (Modifiers::OPT, "Alt"),
    ];
    let modifiers = trigger.modifiers();
    let mut parts: Vec<String> = NAMES
        .iter()
        .filter(|(modifier, _)| modifiers.intersects(*modifier))
        .map(|(_, name)| (*name).to_owned())
        .collect();
    match trigger {
        Trigger::Keys(hotkey) => match hotkey.key {
            Some(Key::Escape) => parts.push("Esc".into()),
            Some(Key::Return) => parts.push("Enter".into()),
            Some(Key::Delete) => parts.push("Backspace".into()),
            Some(Key::ForwardDelete) => parts.push("Delete".into()),
            Some(Key::CapsLock) => parts.push(t!("name.key.capsLock")),
            Some(Key::Space) => parts.push(t!("name.key.space")),
            Some(key) => parts.push(key.to_string()),
            None => {}
        },
        Trigger::Mouse { button, .. } => parts.push(button.label()),
    }
    parts.join("+")
}

/// Shortcuts macOS or every app relies on (KEYS-05).
pub fn macos_reserved(trigger: &Trigger) -> bool {
    let Trigger::Keys(hotkey) = trigger else {
        return false;
    };
    let only = |wanted: Modifiers| only_modifiers(hotkey.modifiers, wanted);
    matches!(
        hotkey.key,
        Some(Key::Delete | Key::ForwardDelete | Key::Space | Key::C | Key::V)
    ) && only(Modifiers::CMD)
        || hotkey.key == Some(Key::F) && only(Modifiers::CMD | Modifiers::CTRL)
}

/// Shortcuts Windows or every app relies on (WIN-03): Ctrl alone, Win with
/// the shell's letters and arrows, Ctrl+C and Ctrl+V.
pub fn windows_reserved(trigger: &Trigger) -> bool {
    let Trigger::Keys(hotkey) = trigger else {
        return false;
    };
    let only = |wanted: Modifiers| only_modifiers(hotkey.modifiers, wanted);
    let shell_keys = [
        Key::E,
        Key::R,
        Key::L,
        Key::D,
        Key::Tab,
        Key::I,
        Key::S,
        Key::X,
        Key::P,
        Key::UpArrow,
        Key::DownArrow,
        Key::Q,
    ];
    match hotkey.key {
        None => only(Modifiers::CTRL),
        Some(key) if shell_keys.contains(&key) && only(Modifiers::CMD) => true,
        Some(Key::C | Key::V) => only(Modifiers::CTRL),
        _ => false,
    }
}

/// Exactly the modifier groups of `wanted`, either side.
fn only_modifiers(modifiers: Modifiers, wanted: Modifiers) -> bool {
    super::trigger::MODIFIER_GROUPS
        .iter()
        .all(|group| modifiers.intersects(*group) == wanted.intersects(*group))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label_of(action: Action) -> String {
        let binding = macos_defaults(true)
            .into_iter()
            .find(|binding| binding.action == action)
            .unwrap();
        macos_label(&binding.trigger)
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
        assert!(blocked_buttons(&[through]).is_empty());
        assert_eq!(blocked.len(), 8);
        assert!(blocked.contains(&Hotkey {
            modifiers: Modifiers::OPT | Modifiers::SHIFT,
            key: Some(Key::Return)
        }));
    }

    #[test]
    fn without_an_fn_key_ctrl_opt_takes_its_place() {
        let label = |action| {
            let binding = macos_defaults(false)
                .into_iter()
                .find(|binding: &Binding| binding.action == action)
                .unwrap();
            macos_label(&binding.trigger)
        };
        assert_eq!(label(Action::PushToTalk), "⌃⌥");
        assert_eq!(label(Action::HandsFree), "⌃⌥Space");
        assert_eq!(label(Action::CommandMode), "⌃⌥⌘");
        assert_eq!(label(Action::PasteLast), "⌃⌘V");
        assert_eq!(label(Action::Cancel), "Esc");
    }

    #[test]
    fn labels_mouse_buttons_and_caps_lock() {
        let label = |text: &str| macos_label(&text.parse::<Trigger>().unwrap());
        assert_eq!(label("MouseMiddle"), "Middle Click");
        assert_eq!(label("Ctrl+Mouse4"), "⌃ Mouse 4");
        assert_eq!(label("Fn+Mouse10"), "fn Mouse 10");
        assert_eq!(label("CapsLock"), "Caps Lock");
    }

    #[test]
    fn mouse_bindings_block_their_button() {
        let binding = Binding::new(
            Action::PushToTalk,
            "Ctrl+Mouse5".parse::<Trigger>().unwrap(),
        );
        assert!(blocked_hotkeys(&[binding]).is_empty());
        assert_eq!(
            blocked_buttons(&[binding]).into_iter().collect::<Vec<_>>(),
            vec![MouseButton::new(4).unwrap()]
        );
    }

    #[test]
    fn every_action_has_exactly_one_default() {
        for defaults in [macos_defaults(true), windows_defaults()] {
            for action in [
                Action::PushToTalk,
                Action::HandsFree,
                Action::Cancel,
                Action::PasteLast,
                Action::CopyLast,
                Action::CommandMode,
                Action::Polish,
                Action::PromptEngineer,
                Action::ViewChanges,
            ] {
                assert_eq!(defaults.iter().filter(|b| b.action == action).count(), 1);
            }
        }
    }

    #[test]
    fn labels_the_windows_defaults() {
        let label = |action| {
            let binding = windows_defaults()
                .into_iter()
                .find(|binding: &Binding| binding.action == action)
                .unwrap();
            windows_label(&binding.trigger)
        };
        assert_eq!(label(Action::PushToTalk), "Ctrl+Win");
        assert_eq!(label(Action::HandsFree), "Ctrl+Win+Space");
        assert_eq!(label(Action::CommandMode), "Ctrl+Win+Alt");
        assert_eq!(label(Action::PasteLast), "Shift+Alt+Z");
        assert_eq!(label(Action::CopyLast), "Shift+Alt+X");
        assert_eq!(label(Action::Polish), "Win+Alt+1");
        assert_eq!(label(Action::PromptEngineer), "Win+Alt+2");
        assert_eq!(label(Action::ViewChanges), "Win+Alt+O");
        assert_eq!(label(Action::Cancel), "Esc");
        let label = |text: &str| windows_label(&text.parse::<Trigger>().unwrap());
        assert_eq!(label("Ctrl+Mouse4"), "Ctrl+Mouse 4");
        assert_eq!(label("MouseMiddle"), "Middle Click");
        assert_eq!(label("CapsLock"), "Caps Lock");
        assert_eq!(label("Opt+Return"), "Alt+Enter");
    }

    #[test]
    fn windows_reserves_its_shell_shortcuts() {
        let reserved = |text: &str| windows_reserved(&text.parse::<Trigger>().unwrap());
        for text in [
            "Ctrl", "Win+E", "Win+R", "Win+L", "Win+D", "Win+Tab", "Win+I", "Win+S", "Win+X",
            "Win+P", "Win+Up", "Win+Down", "Win+Q", "Ctrl+C", "Ctrl+V",
        ] {
            assert!(reserved(text), "{text}");
        }
        for text in [
            "Ctrl+Win",
            "Win+Alt+E",
            "Ctrl+Shift+V",
            "Shift+Alt+Z",
            "Win+Alt+1",
        ] {
            assert!(!reserved(text), "{text}");
        }
        assert!(!macos_reserved(&"Cmd+E".parse::<Trigger>().unwrap()));
    }
}
