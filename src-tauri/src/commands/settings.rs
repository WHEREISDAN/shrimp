use std::sync::Arc;

use serde_json::{Value, json};
use tauri::State;

use super::CommandError;
use crate::hotkeys::{Action, Binding, validation};
use crate::settings::{Settings, SettingsError, SettingsStore};

/// The platform's default bindings, which user shortcuts may not reuse.
pub struct DefaultBindings(pub Vec<Binding>);

#[tauri::command]
pub fn get_settings(settings: State<'_, Arc<SettingsStore>>) -> Settings {
    settings.get()
}

/// Applies a JSON merge patch, e.g. `{"ai": {"cleanup": "medium"}}`, and
/// returns the saved settings. Also emitted as `settings://changed`.
#[tauri::command]
pub fn update_settings(
    patch: Value,
    settings: State<'_, Arc<SettingsStore>>,
) -> Result<Settings, CommandError> {
    settings.update(&patch).map_err(settings_error)
}

fn settings_error(error: SettingsError) -> CommandError {
    match error {
        SettingsError::Invalid(message) => CommandError::new("invalid", message),
        SettingsError::Io(error) => CommandError::new("io", error),
    }
}

/// Records a shortcut the user set (in handy-keys notation, e.g.
/// "Opt+Return"), or clears it with `null`. Checks it against every other
/// shortcut first; the error is the message to show.
#[tauri::command]
pub fn set_shortcut(
    action: Action,
    shortcut: Option<String>,
    settings: State<'_, Arc<SettingsStore>>,
    defaults: State<'_, DefaultBindings>,
) -> Result<Settings, CommandError> {
    let current = settings.get();
    if let Some(text) = &shortcut {
        let taken = taken_shortcuts(&defaults.0, &current, action);
        validation::validate(text, action, &taken)
            .map_err(|message| CommandError::new("invalid", message))?;
    }
    let patch = match action {
        Action::PressEnter => json!({"shortcuts": {"pressEnter": shortcut}}),
        Action::Polish => json!({"transforms": {"polish": {"shortcut": shortcut}}}),
        Action::PromptEngineer => json!({"transforms": {"promptEngineer": {"shortcut": shortcut}}}),
        Action::ViewChanges => json!({"transforms": {"viewChangesShortcut": shortcut}}),
        Action::CustomTransform(index) => {
            let mut custom = current.transforms.custom.clone();
            let transform = custom
                .get_mut(usize::from(index))
                .ok_or_else(|| CommandError::new("notFound", "That transform no longer exists."))?;
            transform.shortcut = shortcut;
            json!({"transforms": {"custom": custom}})
        }
        other => {
            return Err(CommandError::new(
                "unsupported",
                format!("{} can't be changed yet.", other.label()),
            ));
        }
    };
    settings.update(&patch).map_err(settings_error)
}

/// The default bindings and every user shortcut except `action`'s own.
fn taken_shortcuts(defaults: &[Binding], settings: &Settings, action: Action) -> Vec<Binding> {
    let mut user: Vec<(Action, &str)> = settings.transforms.shortcuts();
    if let Some(shortcut) = &settings.shortcuts.press_enter {
        user.push((Action::PressEnter, shortcut));
    }
    let user_bindings = user
        .into_iter()
        .filter(|(other, _)| *other != action)
        .filter_map(|(other, text)| text.parse().ok().map(|hotkey| Binding::new(other, hotkey)));
    defaults.iter().copied().chain(user_bindings).collect()
}
