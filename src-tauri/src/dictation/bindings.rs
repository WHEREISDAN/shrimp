//! The shortcuts the keyboard hook holds at each moment: the platform
//! defaults, Esc only while recording (and Backspace too in Command Mode,
//! CMD-04), and the shortcuts the user set (Press Enter, CORE-15, and the
//! transforms once they are turned on).

use handy_keys::Hotkey;

use crate::hotkeys::{Action, Binding, backspace_cancel};
use crate::settings::Settings;

/// What the controller is doing, as far as bindings go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Moment {
    /// Recording, or a cancel key is still held down.
    pub cancelable: bool,
    /// The session is Command Mode.
    pub command: bool,
}

pub fn active(defaults: &[Binding], settings: &Settings, moment: Moment) -> Vec<Binding> {
    let mut bindings: Vec<Binding> = defaults
        .iter()
        .filter(|binding| moment.cancelable || binding.action != Action::Cancel)
        .copied()
        .collect();
    if moment.cancelable && moment.command {
        bindings.push(backspace_cancel());
    }
    if let Some(hotkey) = settings
        .shortcuts
        .press_enter
        .as_deref()
        .and_then(|text| text.parse::<Hotkey>().ok())
    {
        bindings.push(Binding::with_passthrough(Action::PressEnter, hotkey));
    }
    let transforms = &settings.transforms;
    if transforms.enabled {
        let complete = |action: Action| match action {
            Action::CustomTransform(index) => transforms
                .custom
                .get(usize::from(index))
                .is_some_and(|transform| transform.is_complete()),
            _ => true,
        };
        bindings.extend(
            transforms
                .shortcuts()
                .into_iter()
                .filter(|(action, _)| complete(*action))
                .filter_map(|(action, text)| {
                    text.parse::<Hotkey>()
                        .ok()
                        .map(|hotkey| Binding::new(action, hotkey))
                }),
        );
    }
    bindings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkeys::macos_defaults;

    fn actions(bindings: &[Binding]) -> Vec<Action> {
        bindings.iter().map(|binding| binding.action).collect()
    }

    #[test]
    fn cancel_keys_are_bound_only_while_recording() {
        let settings = Settings::default();
        let idle = active(&macos_defaults(), &settings, Moment::default());
        assert!(!actions(&idle).contains(&Action::Cancel));
        let dictating = Moment {
            cancelable: true,
            command: false,
        };
        let bindings = active(&macos_defaults(), &settings, dictating);
        assert_eq!(
            actions(&bindings)
                .iter()
                .filter(|a| **a == Action::Cancel)
                .count(),
            1
        );
        let commanding = Moment {
            cancelable: true,
            command: true,
        };
        let bindings = active(&macos_defaults(), &settings, commanding);
        assert_eq!(
            actions(&bindings)
                .iter()
                .filter(|a| **a == Action::Cancel)
                .count(),
            2,
            "Esc and Backspace"
        );
    }

    #[test]
    fn press_enter_is_bound_when_set_with_pass_through() {
        let mut settings = Settings::default();
        assert!(
            !actions(&active(&macos_defaults(), &settings, Moment::default()))
                .contains(&Action::PressEnter)
        );
        settings.shortcuts.press_enter = Some("Opt+Return".into());
        let bindings = active(&macos_defaults(), &settings, Moment::default());
        let press_enter = bindings
            .iter()
            .find(|binding| binding.action == Action::PressEnter)
            .unwrap();
        assert!(press_enter.passthrough);
    }

    #[test]
    fn transform_shortcuts_are_bound_once_transforms_are_on() {
        let mut settings = Settings::default();
        let bound =
            |settings: &Settings| actions(&active(&macos_defaults(), settings, Moment::default()));
        assert!(!bound(&settings).contains(&Action::Polish));
        settings.transforms.enabled = true;
        settings
            .transforms
            .custom
            .push(crate::settings::CustomTransform {
                id: "draft".into(),
                shortcut: Some("Opt+3".into()),
                ..Default::default()
            });
        let actions = bound(&settings);
        assert!(actions.contains(&Action::Polish));
        assert!(actions.contains(&Action::PromptEngineer));
        assert!(actions.contains(&Action::ViewChanges));
        assert!(!actions.contains(&Action::CustomTransform(0)), "incomplete");
    }
}
