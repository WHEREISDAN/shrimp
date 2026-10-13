//! The Shortcuts dialog (KEYS-01..10): list every action's bindings, record
//! a new one through the keyboard and mouse hooks, remove one, or reset.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use super::CommandError;
use crate::dictation::{ControllerHandle, ControllerInput, bindings};
use crate::hotkeys::{Action, Binding, Trigger, validation};
use crate::settings::{Settings, SettingsStore};
use crate::t;

/// The platform's default bindings and how shortcuts are written.
pub struct DefaultBindings {
    pub bindings: Vec<Binding>,
    pub label: fn(&Trigger) -> String,
}

/// One action in the dialog. Mirrors `ShortcutRow` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutRow {
    pub action: Action,
    pub name: String,
    /// Each binding, formatted ("fn", "⌃⌘V", "Mouse 4").
    pub shortcuts: Vec<String>,
    pub max: usize,
    /// Must keep at least one binding.
    pub required: bool,
}

/// Mirrors `ShortcutList` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutList {
    pub rows: Vec<ShortcutRow>,
    /// Hands-free sits inside push to talk (KEYS-07).
    pub warning: Option<String>,
    /// The app whose Secure Keyboard Entry blocks keyed shortcuts (KEYS-09).
    pub secure_input: Option<String>,
}

fn name(action: Action, settings: &Settings) -> String {
    match action {
        Action::CustomTransform(index) => settings
            .transforms
            .custom
            .get(usize::from(index))
            .map(|transform| transform.name.trim().to_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| t!("error.shortcuts.untitledTransform")),
        action => action.label(),
    }
}

pub fn list(
    defaults: &DefaultBindings,
    settings: &Settings,
    secure_input: Option<String>,
) -> ShortcutList {
    let rows = bindings::actions(settings)
        .into_iter()
        .map(|action| ShortcutRow {
            action,
            name: name(action, settings),
            shortcuts: bindings::triggers(action, &defaults.bindings, settings)
                .iter()
                .map(defaults.label)
                .collect(),
            max: action.max_bindings(),
            required: action.required(),
        })
        .collect();
    ShortcutList {
        rows,
        warning: validation::overlap_warning(&bindings::configured(&defaults.bindings, settings)),
        secure_input,
    }
}

#[tauri::command]
pub fn get_shortcuts(
    settings: State<'_, Arc<SettingsStore>>,
    defaults: State<'_, DefaultBindings>,
    controller: State<'_, ControllerHandle>,
) -> ShortcutList {
    list(&defaults, &settings.get(), controller.secure_input())
}

/// Records a binding for `action` through the hooks, replacing the one at
/// `index` or adding one. Progress arrives as `shortcuts://recording`.
#[tauri::command]
pub fn record_shortcut(
    action: Action,
    index: Option<usize>,
    controller: State<'_, ControllerHandle>,
) {
    controller.send(ControllerInput::RecordShortcut { action, index });
}

#[tauri::command]
pub fn cancel_shortcut_recording(controller: State<'_, ControllerHandle>) {
    controller.send(ControllerInput::CancelRecording);
}

/// The trash icon: removes one binding; required actions keep their last.
#[tauri::command]
pub fn remove_shortcut(
    action: Action,
    index: usize,
    settings: State<'_, Arc<SettingsStore>>,
    defaults: State<'_, DefaultBindings>,
) -> Result<(), CommandError> {
    let patch = bindings::without_trigger(&defaults.bindings, &settings.get(), action, index)
        .map_err(|message| CommandError::new("invalid", message))?;
    settings
        .update(&patch)
        .map(|_| ())
        .map_err(|error| CommandError::new("invalid", error))
}

/// Restores the platform defaults (KEYS-08); the dialog confirms first.
#[tauri::command]
pub fn reset_shortcuts(settings: State<'_, Arc<SettingsStore>>) -> Result<(), CommandError> {
    settings
        .update(&bindings::reset())
        .map(|_| ())
        .map_err(|error| CommandError::new("invalid", error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkeys::{macos_defaults, macos_label};

    #[test]
    fn lists_every_action_with_its_formatted_bindings() {
        let defaults = DefaultBindings {
            bindings: macos_defaults(true),
            label: macos_label,
        };
        let list = list(&defaults, &Settings::default(), None);
        let json = serde_json::to_value(&list).unwrap();
        assert_eq!(json["rows"][0]["action"], "pushToTalk");
        assert_eq!(json["rows"][0]["shortcuts"], serde_json::json!(["fn"]));
        assert_eq!(json["rows"][0]["max"], 4);
        assert_eq!(json["rows"][0]["required"], true);
        let cancel = list
            .rows
            .iter()
            .find(|row| row.action == Action::Cancel)
            .unwrap();
        assert_eq!(cancel.max, 1);
        let enter = list
            .rows
            .iter()
            .find(|row| row.action == Action::PressEnter)
            .unwrap();
        assert!(enter.shortcuts.is_empty());
        assert_eq!(json["secureInput"], serde_json::Value::Null);
    }
}
