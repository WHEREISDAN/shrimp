use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use super::CommandError;
use super::permissions::AppPermissions;
use crate::app::events::{NAVIGATE_EVENT, TauriUi};
use crate::app::{flow_bar, window};
use crate::dictation::gestures::BarControl;
use crate::dictation::{ControllerHandle, ControllerInput, DictationState, TransformAction};
use crate::hotkeys::Action;
use crate::notify::NoticeAction;
use crate::platform::{self, Permission};
use crate::stt::ModelManager;

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

/// The Flow Bar's content size in logical points; the window follows it.
#[tauri::command]
pub fn flow_bar_layout(width: f64, height: f64, app: AppHandle) {
    flow_bar::resize(&app, width, height);
}

#[tauri::command]
pub fn run_notice_action(
    action: NoticeAction,
    app: AppHandle,
    controller: State<'_, ControllerHandle>,
    permissions: State<'_, AppPermissions>,
    models: State<'_, Arc<ModelManager>>,
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
            tauri::async_runtime::spawn(Arc::clone(&models).prepare_default());
            Ok(())
        }
        NoticeAction::OpenAiSettings => {
            window::show_main(&app);
            let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "settings");
            Ok(())
        }
        NoticeAction::OpenDictionary => {
            window::show_main(&app);
            let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "dictionary");
            Ok(())
        }
        NoticeAction::ViewTransformChanges => {
            controller.send(ControllerInput::Transform(TransformAction::View));
            Ok(())
        }
        NoticeAction::UndoCancel => {
            controller.send(ControllerInput::UndoCancel);
            Ok(())
        }
        NoticeAction::OpenExperimentalSettings => {
            window::show_main(&app);
            let _ = app.emit_to(window::MAIN_LABEL, NAVIGATE_EVENT, "experimental");
            Ok(())
        }
    }
}

/// The changes view's buttons: copy the result, retry or undo the transform.
#[tauri::command]
pub fn transform_action(action: TransformAction, controller: State<'_, ControllerHandle>) {
    controller.send(ControllerInput::Transform(action));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_controls_deserialize_from_camel_case() {
        let control: BarControl = serde_json::from_str("\"confirm\"").unwrap();
        assert_eq!(control, BarControl::Confirm);
    }
}
