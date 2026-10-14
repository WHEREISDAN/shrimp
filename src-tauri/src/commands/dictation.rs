use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use super::CommandError;
use super::permissions::AppPermissions;
use crate::app::events::{NAVIGATE_EVENT, TauriUi};
use crate::app::{flow_bar, updates, window};
use crate::dictation::gestures::BarControl;
use crate::dictation::{
    ControllerHandle, ControllerInput, DictationState, DictationUi, TransformAction,
};
use crate::hotkeys::Action;
use crate::mic::Microphones;
use crate::notify::{Notice, NoticeAction};
use crate::platform::{self, Permission};
use crate::settings::SettingsStore;
use crate::stt::ModelManager;
use crate::t;

#[tauri::command]
pub fn get_dictation_state(controller: State<'_, ControllerHandle>) -> DictationState {
    controller.state()
}

/// Flow Bar clicks: start (hands-free), stop, cancel and confirm.
#[tauri::command]
pub fn flow_bar_control(control: BarControl, controller: State<'_, ControllerHandle>) {
    controller.bar(control);
}

#[tauri::command]
pub fn flow_bar_ready(ui: State<'_, Arc<TauriUi>>) {
    ui.flow_bar_ready();
}

/// The Flow Bar's content size in logical points; the window follows it,
/// and hides at 0 x 0 (BAR-04).
#[tauri::command]
pub fn flow_bar_layout(width: f64, height: f64, app: AppHandle) {
    flow_bar::resize(&app, width, height);
}

/// Dragging the resting bar: it follows the cursor until `ended`, then
/// docks to the nearest edge (BAR-05).
#[tauri::command]
pub fn flow_bar_drag(ended: bool, app: AppHandle) {
    flow_bar::drag(&app, ended);
}

/// The Flow Bar's right-click menu (BAR-06). The microphone list uses the
/// microphone commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlowBarAction {
    PasteLast,
    History,
    Settings,
    /// Hide for 1 hour (BAR-04).
    Hide,
}

const HIDE_FOR_SECONDS: u64 = 60 * 60;

#[tauri::command]
pub fn flow_bar_action(
    action: FlowBarAction,
    app: AppHandle,
    controller: State<'_, ControllerHandle>,
    settings: State<'_, Arc<SettingsStore>>,
    ui: State<'_, Arc<TauriUi>>,
) -> Result<(), CommandError> {
    let navigate = |section: &str| {
        window::show_main(&app);
        let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, section);
    };
    match action {
        FlowBarAction::PasteLast => controller.send(ControllerInput::Menu(Action::PasteLast)),
        FlowBarAction::History => navigate("history"),
        FlowBarAction::Settings => navigate("settings"),
        FlowBarAction::Hide => {
            let until = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_secs())
                + HIDE_FOR_SECONDS;
            settings
                .update(&serde_json::json!({"flowBar": {"hiddenUntil": until}}))
                .map_err(|err| CommandError::new("invalid", err))?;
            ui.notice(
                Notice::info(t!("notice.flowBar.hidden")).with_action(NoticeAction::ShowFlowBar),
            );
        }
    }
    Ok(())
}

#[tauri::command]
pub fn run_notice_action(
    action: NoticeAction,
    app: AppHandle,
    controller: State<'_, ControllerHandle>,
    permissions: State<'_, AppPermissions>,
    models: State<'_, Arc<ModelManager>>,
    settings: State<'_, Arc<SettingsStore>>,
    microphones: State<'_, Arc<Microphones>>,
) -> Result<(), CommandError> {
    let open = |permission| {
        permissions
            .0
            .open_settings(permission)
            .map_err(|err| CommandError::new("platform", err))
    };
    match action {
        NoticeAction::CopyLastTranscript => {
            controller.send(ControllerInput::Menu(Action::CopyLast));
            Ok(())
        }
        NoticeAction::OpenAccessibilitySettings => open(Permission::Accessibility),
        NoticeAction::OpenMicrophoneSettings => open(Permission::Microphone),
        NoticeAction::OpenKeyboardSettings => {
            platform::open_keyboard_settings().map_err(|err| CommandError::new("platform", err))
        }
        NoticeAction::RetryModelDownload => {
            let chosen = settings.get().stt.local_model;
            tauri::async_runtime::spawn(Arc::clone(&models).prepare(chosen));
            Ok(())
        }
        NoticeAction::OpenAiSettings => {
            window::show_main(&app);
            let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "ai");
            Ok(())
        }
        NoticeAction::OpenDictionary => {
            window::show_main(&app);
            let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "dictionary");
            Ok(())
        }
        NoticeAction::ViewTransformChanges | NoticeAction::RevealEdit => {
            controller.send(ControllerInput::Transform(TransformAction::View));
            Ok(())
        }
        NoticeAction::ChooseMicrophone | NoticeAction::SwitchMicrophone => {
            window::show_main(&app);
            let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "microphone");
            Ok(())
        }
        NoticeAction::SwitchToNewMicrophone | NoticeAction::KeepMicrophone => microphones
            .answer_new_microphone(action == NoticeAction::SwitchToNewMicrophone)
            .map_err(|message| CommandError::new("invalid", message)),
        NoticeAction::DismissBluetoothAdvice => microphones
            .dismiss_bluetooth_advice()
            .map_err(|message| CommandError::new("invalid", message)),
        NoticeAction::OpenSoundSettings | NoticeAction::TroubleshootMicrophone => {
            platform::open_sound_settings().map_err(|err| CommandError::new("platform", err))
        }
        NoticeAction::OpenLanguageSettings => {
            window::show_main(&app);
            let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "languages");
            Ok(())
        }
        NoticeAction::AddDetectedLanguage => {
            controller.send(ControllerInput::AddDetectedLanguage);
            Ok(())
        }
        NoticeAction::DismissLanguageCard => settings
            .update(&serde_json::json!({"notices": {"languageCardOff": true}}))
            .map(|_| ())
            .map_err(|err| CommandError::new("invalid", err)),
        NoticeAction::ShowFlowBar => settings
            .update(&serde_json::json!({"flowBar": {"hiddenUntil": 0}}))
            .map(|_| ())
            .map_err(|err| CommandError::new("invalid", err)),
        NoticeAction::UndoCancel => {
            controller.send(ControllerInput::UndoCancel);
            Ok(())
        }
        NoticeAction::OpenExperimentalSettings => {
            window::show_main(&app);
            let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "experimental");
            Ok(())
        }
        NoticeAction::OpenHistory => {
            window::show_main(&app);
            let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "history");
            Ok(())
        }
        NoticeAction::OpenInsights => {
            window::show_main(&app);
            let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "insights");
            Ok(())
        }
        NoticeAction::SeeVoiceProfile | NoticeAction::UpdateVoiceProfile => {
            window::show_main(&app);
            let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "voiceProfile");
            Ok(())
        }
        NoticeAction::RestartToUpdate => {
            updates::restart_when_idle(&app);
            Ok(())
        }
    }
}

/// The changes view's buttons: copy the result, retry or undo the transform.
#[tauri::command]
pub fn transform_action(action: TransformAction, controller: State<'_, ControllerHandle>) {
    controller.send(ControllerInput::Transform(action));
}

/// The Flow Bar's wand and its menu (TRF-07). Mirrors `WandAction` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WandAction {
    /// Runs the chosen transform on the app in front, which keeps focus.
    Run,
    /// Chooses the transform the wand (and Auto Apply, when on) runs.
    Choose(String),
    /// Turns Auto Apply on with the chosen transform, or off.
    AutoApply(bool),
    /// Opens the Transforms page.
    Configure,
}

#[tauri::command]
pub fn flow_bar_wand(
    action: WandAction,
    app: AppHandle,
    controller: State<'_, ControllerHandle>,
    settings: State<'_, Arc<SettingsStore>>,
) -> Result<(), CommandError> {
    let transforms = settings.get().transforms;
    let patch = match action {
        WandAction::Run => {
            controller.send(ControllerInput::Wand);
            return Ok(());
        }
        WandAction::Configure => {
            window::show_main(&app);
            let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "transforms");
            return Ok(());
        }
        WandAction::Choose(id) => transforms
            .choose_wand(&id)
            .map_err(|message| CommandError::new("invalid", message))?,
        WandAction::AutoApply(on) => transforms.set_auto_apply(on),
    };
    settings
        .update(&patch)
        .map(|_| ())
        .map_err(|err| CommandError::new("invalid", err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_controls_deserialize_from_camel_case() {
        let control: BarControl = serde_json::from_str("\"confirm\"").unwrap();
        assert_eq!(control, BarControl::Confirm);
    }

    #[test]
    fn wand_actions_deserialize_from_the_flow_bar() {
        let parse = |json| serde_json::from_value::<WandAction>(json).unwrap();
        assert_eq!(parse(serde_json::json!("run")), WandAction::Run);
        assert_eq!(parse(serde_json::json!("configure")), WandAction::Configure);
        assert_eq!(
            parse(serde_json::json!({"choose": "promptEngineer"})),
            WandAction::Choose("promptEngineer".into())
        );
        assert_eq!(
            parse(serde_json::json!({"autoApply": true})),
            WandAction::AutoApply(true)
        );
    }
}
