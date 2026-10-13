//! The shortcuts bound at each moment: the user's bindings or the platform
//! defaults per action, Esc only while recording (and Backspace too in
//! Command Mode, CMD-04), and the transforms once they are turned on. Also
//! the settings patches that add, replace and remove bindings (KEYS-01..08).

use serde_json::{Value, json};

use crate::hotkeys::{Action, Binding, Trigger, backspace_cancel, validation};
use crate::settings::Settings;
use crate::settings::shortcuts::key_for_action;

/// What the controller is doing, as far as bindings go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Moment {
    /// Recording, or a cancel key is still held down.
    pub cancelable: bool,
    /// The session is Command Mode.
    pub command: bool,
}

const CORE_ACTIONS: [Action; 7] = [
    Action::PushToTalk,
    Action::HandsFree,
    Action::CommandMode,
    Action::PasteLast,
    Action::CopyLast,
    Action::Cancel,
    Action::PressEnter,
];

/// Every action with shortcuts, in the order the Shortcuts dialog lists
/// them; transforms only once they are turned on.
pub fn actions(settings: &Settings) -> Vec<Action> {
    let mut actions = CORE_ACTIONS.to_vec();
    let transforms = &settings.transforms;
    if transforms.enabled {
        actions.extend([Action::Polish, Action::PromptEngineer]);
        actions.extend(
            (0..transforms.custom.len())
                .filter_map(|index| u8::try_from(index).ok())
                .map(Action::CustomTransform),
        );
        actions.push(Action::ViewChanges);
    }
    actions
}

fn default_triggers(action: Action, defaults: &[Binding]) -> Vec<Trigger> {
    let fixed = |text: &str| text.parse::<Trigger>().into_iter().collect();
    match action {
        Action::Polish => fixed("Opt+1"),
        Action::PromptEngineer => fixed("Opt+2"),
        Action::ViewChanges => fixed("Opt+O"),
        Action::PressEnter | Action::CustomTransform(_) => Vec::new(),
        action => defaults
            .iter()
            .filter(|binding| binding.action == action)
            .map(|binding| binding.trigger)
            .collect(),
    }
}

/// The triggers of `action`: the user's, or its defaults.
pub fn triggers(action: Action, defaults: &[Binding], settings: &Settings) -> Vec<Trigger> {
    let stored = key_for_action(action, &settings.transforms)
        .and_then(|key| settings.shortcuts.bindings.get(&key));
    match stored {
        Some(list) => list.iter().filter_map(|text| text.parse().ok()).collect(),
        None => default_triggers(action, defaults),
    }
}

fn binding(action: Action, trigger: Trigger) -> Binding {
    if action == Action::PressEnter {
        Binding::with_passthrough(action, trigger)
    } else {
        Binding::new(action, trigger)
    }
}

/// Every configured binding, bound or not at this moment.
pub fn configured(defaults: &[Binding], settings: &Settings) -> Vec<Binding> {
    actions(settings)
        .into_iter()
        .filter(|action| match action {
            Action::CustomTransform(index) => settings
                .transforms
                .custom
                .get(usize::from(*index))
                .is_some_and(|transform| transform.is_complete()),
            _ => true,
        })
        .flat_map(|action| {
            triggers(action, defaults, settings)
                .into_iter()
                .map(move |trigger| binding(action, trigger))
        })
        .collect()
}

pub fn active(defaults: &[Binding], settings: &Settings, moment: Moment) -> Vec<Binding> {
    let mut bindings: Vec<Binding> = configured(defaults, settings)
        .into_iter()
        .filter(|binding| moment.cancelable || binding.action != Action::Cancel)
        .collect();
    if moment.cancelable && moment.command {
        bindings.push(backspace_cancel());
    }
    bindings
}

/// A recorded trigger, checked against every other binding, as the patch
/// that saves it at `index` (or adds it), and the overlap warning if any.
pub fn with_trigger(
    defaults: &[Binding],
    settings: &Settings,
    action: Action,
    index: Option<usize>,
    trigger: Trigger,
    label: fn(&Trigger) -> String,
) -> Result<(Value, Option<&'static str>), String> {
    let mut list = triggers(action, defaults, settings);
    let others: Vec<Binding> = configured(defaults, settings)
        .into_iter()
        .filter(|other| other.action != action)
        .chain(
            list.iter()
                .enumerate()
                .filter(|(position, _)| Some(*position) != index)
                .map(|(_, trigger)| Binding::new(action, *trigger)),
        )
        .collect();
    validation::validate(&trigger, action, &others, label)?;
    match index.filter(|index| *index < list.len()) {
        Some(index) => list[index] = trigger,
        None if list.len() >= action.max_bindings() => {
            return Err(format!(
                "{} takes at most {} shortcuts.",
                action.label(),
                action.max_bindings()
            ));
        }
        None => list.push(trigger),
    }
    let after: Vec<Binding> = others
        .into_iter()
        .filter(|other| other.action != action)
        .chain(list.iter().map(|trigger| Binding::new(action, *trigger)))
        .collect();
    Ok((
        patch(settings, action, &list)?,
        validation::overlap_warning(&after),
    ))
}

/// The patch that removes the binding at `index` (the trash icon).
pub fn without_trigger(
    defaults: &[Binding],
    settings: &Settings,
    action: Action,
    index: usize,
) -> Result<Value, String> {
    let mut list = triggers(action, defaults, settings);
    if index >= list.len() {
        return Err("That shortcut is already gone.".into());
    }
    if action.required() && list.len() == 1 {
        return Err(format!("{} needs at least one shortcut.", action.label()));
    }
    list.remove(index);
    patch(settings, action, &list)
}

/// Back to the platform defaults, unbound-by-default actions included (KEYS-08).
pub fn reset() -> Value {
    json!({"shortcuts": {"bindings": null}})
}

fn patch(settings: &Settings, action: Action, list: &[Trigger]) -> Result<Value, String> {
    let key = key_for_action(action, &settings.transforms)
        .ok_or_else(|| "That transform no longer exists.".to_owned())?;
    let texts: Vec<String> = list.iter().map(Trigger::to_string).collect();
    Ok(json!({"shortcuts": {"bindings": {key: texts}}}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkeys::{macos_defaults, macos_label};
    use crate::settings::SettingsStore;

    fn actions_of(bindings: &[Binding]) -> Vec<Action> {
        bindings.iter().map(|binding| binding.action).collect()
    }

    fn trigger(text: &str) -> Trigger {
        text.parse().unwrap()
    }

    #[test]
    fn cancel_keys_are_bound_only_while_recording() {
        let settings = Settings::default();
        let defaults = macos_defaults(true);
        let idle = active(&defaults, &settings, Moment::default());
        assert!(!actions_of(&idle).contains(&Action::Cancel));
        let dictating = Moment {
            cancelable: true,
            command: false,
        };
        let count = |bindings: &[Binding]| {
            actions_of(bindings)
                .iter()
                .filter(|action| **action == Action::Cancel)
                .count()
        };
        assert_eq!(count(&active(&defaults, &settings, dictating)), 1);
        let commanding = Moment {
            cancelable: true,
            command: true,
        };
        assert_eq!(
            count(&active(&defaults, &settings, commanding)),
            2,
            "Esc and Backspace"
        );
    }

    #[test]
    fn press_enter_is_unbound_by_default_and_passes_modifiers_through() {
        let defaults = macos_defaults(true);
        let store = SettingsStore::in_memory(Settings::default());
        assert!(!actions_of(&configured(&defaults, &store.get())).contains(&Action::PressEnter));
        let (patch, _) = with_trigger(
            &defaults,
            &store.get(),
            Action::PressEnter,
            None,
            trigger("Opt+Return"),
            macos_label,
        )
        .unwrap();
        let settings = store.update(&patch).unwrap();
        let bindings = configured(&defaults, &settings);
        let press_enter = bindings
            .iter()
            .find(|binding| binding.action == Action::PressEnter)
            .unwrap();
        assert!(press_enter.passthrough);
    }

    #[test]
    fn transform_shortcuts_are_bound_once_transforms_are_on() {
        let mut settings = Settings::default();
        let defaults = macos_defaults(true);
        let bound =
            |settings: &Settings| actions_of(&active(&defaults, settings, Moment::default()));
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
        settings.normalize();
        let actions = bound(&settings);
        assert!(actions.contains(&Action::Polish));
        assert!(actions.contains(&Action::PromptEngineer));
        assert!(actions.contains(&Action::ViewChanges));
        assert!(!actions.contains(&Action::CustomTransform(0)), "incomplete");
        assert_eq!(
            triggers(Action::CustomTransform(0), &defaults, &settings),
            vec![trigger("Opt+3")]
        );
    }

    #[test]
    fn actions_take_up_to_four_bindings_and_keep_their_required_one() {
        let defaults = macos_defaults(true);
        let store = SettingsStore::in_memory(Settings::default());
        for text in ["Ctrl+Opt", "Mouse4", "CapsLock"] {
            let (patch, warning) = with_trigger(
                &defaults,
                &store.get(),
                Action::PushToTalk,
                None,
                trigger(text),
                macos_label,
            )
            .unwrap();
            assert_eq!(warning, None);
            store.update(&patch).unwrap();
        }
        let settings = store.get();
        assert_eq!(
            triggers(Action::PushToTalk, &defaults, &settings).len(),
            4,
            "fn and three more"
        );
        let fifth = with_trigger(
            &defaults,
            &settings,
            Action::PushToTalk,
            None,
            trigger("Mouse5"),
            macos_label,
        );
        assert!(fifth.unwrap_err().contains("at most 4"));
        let replaced = with_trigger(
            &defaults,
            &settings,
            Action::PushToTalk,
            Some(3),
            trigger("Mouse5"),
            macos_label,
        );
        assert!(replaced.is_ok());
        let duplicate = with_trigger(
            &defaults,
            &settings,
            Action::PushToTalk,
            Some(3),
            trigger("Mouse4"),
            macos_label,
        );
        assert_eq!(duplicate.unwrap_err(), validation::IN_USE);

        for _ in 0..3 {
            let patch = without_trigger(&defaults, &store.get(), Action::PushToTalk, 0).unwrap();
            store.update(&patch).unwrap();
        }
        let last = without_trigger(&defaults, &store.get(), Action::PushToTalk, 0);
        assert!(last.unwrap_err().contains("at least one"));
        let patch = without_trigger(&defaults, &store.get(), Action::HandsFree, 0).unwrap();
        let settings = store.update(&patch).unwrap();
        assert!(triggers(Action::HandsFree, &defaults, &settings).is_empty());

        let settings = store.update(&reset()).unwrap();
        assert_eq!(
            triggers(Action::PushToTalk, &defaults, &settings),
            vec![trigger("Fn")]
        );
        assert_eq!(
            triggers(Action::HandsFree, &defaults, &settings),
            vec![trigger("Fn+Space")]
        );
    }

    #[test]
    fn a_hands_free_part_of_push_to_talk_saves_with_a_warning() {
        let defaults = macos_defaults(true);
        let store = SettingsStore::in_memory(Settings::default());
        let (patch, _) = with_trigger(
            &defaults,
            &store.get(),
            Action::PushToTalk,
            Some(0),
            trigger("Ctrl+Opt+Space"),
            macos_label,
        )
        .unwrap();
        store.update(&patch).unwrap();
        let (_, warning) = with_trigger(
            &defaults,
            &store.get(),
            Action::HandsFree,
            Some(0),
            trigger("Ctrl+Opt"),
            macos_label,
        )
        .unwrap();
        assert_eq!(warning, Some(validation::OVERLAP_WARNING));
    }

    #[test]
    fn cancel_takes_one_binding() {
        let defaults = macos_defaults(true);
        let settings = Settings::default();
        let added = with_trigger(
            &defaults,
            &settings,
            Action::Cancel,
            None,
            trigger("Ctrl+Escape"),
            macos_label,
        );
        assert!(added.unwrap_err().contains("at most 1"));
    }
}
