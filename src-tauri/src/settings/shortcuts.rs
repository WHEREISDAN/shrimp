//! The shortcuts the user set (KEYS-01..10). Each action's bindings are
//! stored under its key; an action that is missing uses its defaults, and an
//! empty list means unbound. The file is per device, so shortcuts never
//! leave this Mac.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::transforms::{TransformSettings, custom_key};
use crate::hotkeys::{Action, Trigger};
use crate::{t, tn};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShortcutSettings {
    /// Bindings per action, in trigger notation ("Ctrl+Opt", "Mouse4").
    pub bindings: BTreeMap<String, Vec<String>>,
    /// Phase 3's single Press Enter shortcut, moved into `bindings` on load.
    #[serde(skip_serializing)]
    pub press_enter: Option<String>,
}

const CORE_ACTIONS: [Action; 10] = [
    Action::PushToTalk,
    Action::HandsFree,
    Action::CommandMode,
    Action::PasteLast,
    Action::CopyLast,
    Action::Cancel,
    Action::PressEnter,
    Action::Polish,
    Action::PromptEngineer,
    Action::ViewChanges,
];

impl ShortcutSettings {
    /// Moves phase 3's shortcut fields into `bindings` and drops the
    /// bindings of custom transforms that no longer exist.
    pub fn normalize(&mut self, transforms: &mut TransformSettings) {
        let mut legacy = transforms.take_legacy_shortcuts();
        if let Some(shortcut) = self.press_enter.take() {
            legacy.push(("pressEnter".into(), shortcut));
        }
        for (key, shortcut) in legacy {
            self.bindings.entry(key).or_insert_with(|| vec![shortcut]);
        }
        self.bindings
            .retain(|key, _| match key.strip_prefix("transform:") {
                Some(id) => transforms.custom.iter().any(|transform| transform.id == id),
                None => true,
            });
    }

    pub fn validate(&self, transforms: &TransformSettings) -> Result<(), String> {
        for (key, shortcuts) in &self.bindings {
            let action = action_for_key(key, transforms)
                .ok_or_else(|| t!("error.shortcuts.unknownAction", key = key))?;
            if shortcuts.len() > action.max_bindings() {
                return Err(tn!(
                    "error.shortcuts.tooMany",
                    action.max_bindings(),
                    action = action.label()
                ));
            }
            if shortcuts.is_empty() && action.required() {
                return Err(t!("error.shortcuts.required", action = action.label()));
            }
            if let Some(bad) = shortcuts
                .iter()
                .find(|shortcut| shortcut.parse::<Trigger>().is_err())
            {
                return Err(t!("error.shortcuts.invalid", shortcut = bad));
            }
        }
        Ok(())
    }
}

/// The key an action's bindings are stored under.
pub fn key_for_action(action: Action, transforms: &TransformSettings) -> Option<String> {
    match action {
        Action::CustomTransform(index) => transforms
            .custom
            .get(usize::from(index))
            .map(|transform| custom_key(&transform.id)),
        action => action.storage_key().map(str::to_owned),
    }
}

pub fn action_for_key(key: &str, transforms: &TransformSettings) -> Option<Action> {
    if let Some(id) = key.strip_prefix("transform:") {
        let index = transforms.custom.iter().position(|t| t.id == id)?;
        return u8::try_from(index).ok().map(Action::CustomTransform);
    }
    CORE_ACTIONS
        .into_iter()
        .find(|action| action.storage_key() == Some(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::CustomTransform;

    fn transforms() -> TransformSettings {
        TransformSettings {
            custom: vec![CustomTransform {
                id: "pirate".into(),
                ..CustomTransform::default()
            }],
            ..TransformSettings::default()
        }
    }

    #[test]
    fn phase_3_shortcuts_move_into_the_map_without_overriding_it() {
        let mut transforms = transforms();
        transforms.polish.shortcut = Some("Opt+1".into());
        transforms.custom[0].shortcut = Some("Opt+3".into());
        let mut shortcuts = ShortcutSettings {
            press_enter: Some("Opt+Return".into()),
            ..ShortcutSettings::default()
        };
        shortcuts
            .bindings
            .insert("polish".into(), vec!["Opt+9".into()]);
        shortcuts
            .bindings
            .insert("transform:gone".into(), vec!["Opt+4".into()]);
        shortcuts.normalize(&mut transforms);
        assert_eq!(shortcuts.bindings["polish"], ["Opt+9"]);
        assert_eq!(shortcuts.bindings["pressEnter"], ["Opt+Return"]);
        assert_eq!(shortcuts.bindings["transform:pirate"], ["Opt+3"]);
        assert!(!shortcuts.bindings.contains_key("transform:gone"));
        assert_eq!(transforms.custom[0].shortcut, None);
    }

    #[test]
    fn validation_follows_the_per_action_limits() {
        let transforms = transforms();
        let mut shortcuts = ShortcutSettings::default();
        let set = |shortcuts: &mut ShortcutSettings, key: &str, list: &[&str]| {
            shortcuts
                .bindings
                .insert(key.into(), list.iter().map(|s| (*s).to_owned()).collect());
        };
        set(
            &mut shortcuts,
            "pushToTalk",
            &["Fn", "Ctrl+Opt", "Mouse4", "CapsLock"],
        );
        set(&mut shortcuts, "handsFree", &[]);
        set(&mut shortcuts, "transform:pirate", &["Opt+3"]);
        shortcuts.validate(&transforms).unwrap();
        set(&mut shortcuts, "cancel", &["Escape", "Ctrl+Escape"]);
        assert!(shortcuts.validate(&transforms).is_err(), "Cancel takes one");
        set(&mut shortcuts, "cancel", &[]);
        assert!(
            shortcuts.validate(&transforms).is_err(),
            "Cancel is required"
        );
        set(&mut shortcuts, "cancel", &["Escape"]);
        set(&mut shortcuts, "pasteLast", &["not a key+++"]);
        assert!(shortcuts.validate(&transforms).is_err());
    }

    #[test]
    fn maps_actions_to_their_keys() {
        let transforms = transforms();
        assert_eq!(
            key_for_action(Action::CustomTransform(0), &transforms).as_deref(),
            Some("transform:pirate")
        );
        assert_eq!(
            action_for_key("transform:pirate", &transforms),
            Some(Action::CustomTransform(0))
        );
        assert_eq!(
            action_for_key("handsFree", &transforms),
            Some(Action::HandsFree)
        );
        assert_eq!(action_for_key("nope", &transforms), None);
    }
}
