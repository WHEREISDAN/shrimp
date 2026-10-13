use std::sync::Arc;

use serde_json::Value;
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
    let field = match action {
        Action::PressEnter => "pressEnter",
        other => {
            return Err(CommandError::new(
                "unsupported",
                format!("{} can't be changed yet.", other.label()),
            ));
        }
    };
    if let Some(text) = &shortcut {
        validation::validate(text, action, &defaults.0)
            .map_err(|message| CommandError::new("invalid", message))?;
    }
    let patch = serde_json::json!({"shortcuts": {field: shortcut}});
    settings.update(&patch).map_err(settings_error)
}
