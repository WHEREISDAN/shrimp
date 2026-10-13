//! What a binding listens for: a key combination (Fn and modifier-only chords
//! included) or a mouse button, alone or with modifiers. Stored as text such
//! as "Fn+Space", "Ctrl+Opt", "MouseMiddle" or "Ctrl+Mouse4".

use std::fmt;
use std::str::FromStr;

use handy_keys::{Hotkey, Key, Modifiers};

use crate::t;

/// Modifier groups, either side.
pub const MODIFIER_GROUPS: [Modifiers; 5] = [
    Modifiers::CTRL,
    Modifiers::OPT,
    Modifiers::SHIFT,
    Modifiers::CMD,
    Modifiers::FN,
];

/// A mouse button by its Core Graphics number: 2 is the middle button and
/// 3 to 9 are Mouse 4 to Mouse 10. Left and right clicks can't be bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseButton(u8);

impl MouseButton {
    pub const MIDDLE: Self = Self(2);

    pub fn new(number: i64) -> Option<Self> {
        u8::try_from(number)
            .ok()
            .filter(|number| (2..=9).contains(number))
            .map(Self)
    }

    pub fn number(self) -> u8 {
        self.0
    }

    pub fn label(self) -> String {
        match self.0 {
            2 => t!("name.key.middleClick"),
            number => t!("name.key.mouseButton", number = number + 1),
        }
    }

    fn name(self) -> String {
        match self.0 {
            2 => "MouseMiddle".into(),
            number => format!("Mouse{}", number + 1),
        }
    }

    fn parse(text: &str) -> Option<Self> {
        let lower = text.trim().to_ascii_lowercase();
        if lower == "mousemiddle" {
            return Some(Self::MIDDLE);
        }
        let number: u8 = lower.strip_prefix("mouse")?.parse().ok()?;
        (4..=10).contains(&number).then(|| Self(number - 1))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trigger {
    Keys(Hotkey),
    Mouse {
        modifiers: Modifiers,
        button: MouseButton,
    },
}

impl Trigger {
    pub fn keys(modifiers: Modifiers, key: Option<Key>) -> Self {
        Self::Keys(Hotkey { modifiers, key })
    }

    pub fn modifiers(&self) -> Modifiers {
        match self {
            Self::Keys(hotkey) => hotkey.modifiers,
            Self::Mouse { modifiers, .. } => *modifiers,
        }
    }

    pub fn key(&self) -> Option<Key> {
        match self {
            Self::Keys(hotkey) => hotkey.key,
            Self::Mouse { .. } => None,
        }
    }

    pub fn mouse(&self) -> Option<MouseButton> {
        match self {
            Self::Mouse { button, .. } => Some(*button),
            Self::Keys(_) => None,
        }
    }

    /// Modifier groups held plus the key or button: "Ctrl+Opt+Space" is 3.
    pub fn key_count(&self) -> usize {
        let modifiers = MODIFIER_GROUPS
            .iter()
            .filter(|group| self.modifiers().intersects(**group))
            .count();
        let other = match self {
            Self::Keys(hotkey) => usize::from(hotkey.key.is_some()),
            Self::Mouse { .. } => 1,
        };
        modifiers + other
    }

    /// Fires on exactly the same input as `other` (modifiers match either side).
    pub fn same_as(&self, other: &Trigger) -> bool {
        match (self, other) {
            (Self::Keys(a), Self::Keys(b)) => a.key == b.key && a.modifiers.matches(b.modifiers),
            (
                Self::Mouse {
                    modifiers: a,
                    button: x,
                },
                Self::Mouse {
                    modifiers: b,
                    button: y,
                },
            ) => x == y && a.matches(*b),
            _ => false,
        }
    }

    /// Every modifier group and the key of `self` are also in `other`.
    pub fn is_part_of(&self, other: &Trigger) -> bool {
        let groups = |modifiers: Modifiers| {
            MODIFIER_GROUPS
                .iter()
                .filter(move |group| modifiers.intersects(**group))
                .copied()
                .collect::<Vec<_>>()
        };
        let inner = groups(self.modifiers());
        let outer = groups(other.modifiers());
        let input_inside = match (self, other) {
            (Self::Keys(a), Self::Keys(b)) => a.key.is_none() || a.key == b.key,
            (Self::Keys(a), Self::Mouse { .. }) => a.key.is_none(),
            (Self::Mouse { button: a, .. }, Self::Mouse { button: b, .. }) => a == b,
            (Self::Mouse { .. }, Self::Keys(_)) => false,
        };
        input_inside && inner.iter().all(|group| outer.contains(group))
    }
}

impl From<Hotkey> for Trigger {
    fn from(hotkey: Hotkey) -> Self {
        Self::Keys(hotkey)
    }
}

impl fmt::Display for Trigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let modifiers = self.modifiers();
        let tail = match self {
            Self::Keys(hotkey) => hotkey.key.map(|key| key.to_string()),
            Self::Mouse { button, .. } => Some(button.name()),
        };
        let parts: Vec<String> = [(!modifiers.is_empty()).then(|| modifiers.to_string()), tail]
            .into_iter()
            .flatten()
            .collect();
        write!(f, "{}", parts.join("+"))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0} is not a shortcut")]
pub struct ParseTriggerError(String);

impl FromStr for Trigger {
    type Err = ParseTriggerError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let invalid = || ParseTriggerError(text.to_owned());
        let (rest, last) = match text.rsplit_once('+') {
            Some((rest, last)) => (rest, last),
            None => ("", text),
        };
        if let Some(button) = MouseButton::parse(last) {
            let modifiers = rest.parse::<Modifiers>().map_err(|_| invalid())?;
            return Ok(Self::Mouse { modifiers, button });
        }
        let hotkey = text.parse::<Hotkey>().map_err(|_| invalid())?;
        // Mouse buttons are written MouseMiddle and Mouse4 to Mouse10 only.
        let mouse_key = matches!(
            hotkey.key,
            Some(Key::MouseLeft | Key::MouseRight | Key::MouseMiddle | Key::MouseX1 | Key::MouseX2)
        );
        if mouse_key {
            return Err(invalid());
        }
        Ok(Self::Keys(hotkey))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trigger(text: &str) -> Trigger {
        text.parse().unwrap()
    }

    #[test]
    fn parses_and_prints_keys_and_mouse_buttons() {
        for text in [
            "Fn+Space",
            "Ctrl+Opt",
            "MouseMiddle",
            "Ctrl+Mouse4",
            "Mouse10",
        ] {
            assert_eq!(trigger(text).to_string(), text);
        }
        assert_eq!(trigger("Mouse4").mouse(), MouseButton::new(3));
        assert_eq!(trigger("Mouse10").mouse(), MouseButton::new(9));
        assert!("Mouse11".parse::<Trigger>().is_err());
        assert!("Mouse2".parse::<Trigger>().is_err());
        assert_eq!(MouseButton::new(1), None, "right click can't be bound");
    }

    #[test]
    fn counts_keys_by_modifier_group() {
        assert_eq!(trigger("Fn").key_count(), 1);
        assert_eq!(trigger("Ctrl+Opt+Space").key_count(), 3);
        assert_eq!(trigger("Cmd+Ctrl+Opt+K").key_count(), 4);
        assert_eq!(trigger("Shift+Mouse5").key_count(), 2);
    }

    #[test]
    fn tells_when_one_combination_sits_inside_another() {
        assert!(trigger("Ctrl+Opt").is_part_of(&trigger("Ctrl+Opt+Space")));
        assert!(trigger("Fn").is_part_of(&trigger("Fn+Space")));
        assert!(!trigger("Fn+Space").is_part_of(&trigger("Fn")));
        assert!(!trigger("Ctrl+K").is_part_of(&trigger("Ctrl+Opt+J")));
        assert!(trigger("Ctrl").is_part_of(&trigger("Ctrl+Mouse4")));
    }

    #[test]
    fn same_as_ignores_modifier_sides() {
        let left: Trigger = Trigger::keys(Modifiers::CTRL_LEFT, Some(Key::K));
        assert!(trigger("Ctrl+K").same_as(&left));
        assert!(!trigger("Ctrl+K").same_as(&trigger("Ctrl+J")));
        assert!(trigger("Mouse4").same_as(&trigger("Mouse4")));
    }
}
