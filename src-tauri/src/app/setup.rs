//! Builds the app's services once at startup and hands them to Tauri's
//! managed state.

use std::sync::Arc;

use tauri::{App, AppHandle, Emitter, Manager};

use super::events::{MODELS_EVENT, TauriUi};
use super::{flow_bar, tray};
use crate::audio::AudioSourceSpec;
use crate::commands::dictation::{ShortcutInfo, Shortcuts};
use crate::commands::permissions::AppPermissions;
use crate::dictation::{self, DictationUi, Services, Timing};
use crate::hotkeys::Action;
use crate::notify::{Notice, NoticeAction};
use crate::platform::{self, MainThread};
use crate::stt::{EngineSlot, ModelManager, SttOptions};

pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    let platform = platform::native(main_thread(&handle));

    let shortcuts: Vec<ShortcutInfo> = platform
        .default_bindings
        .iter()
        .map(|binding| ShortcutInfo {
            action: binding.action,
            name: binding.action.label(),
            shortcut: (platform.label)(&binding.hotkey),
        })
        .collect();
    let push_to_talk_label = shortcuts
        .iter()
        .find(|info| info.action == Action::PushToTalk)
        .map(|info| info.shortcut.clone())
        .unwrap_or_default();

    let ui = Arc::new(TauriUi::new(handle.clone()));
    if let Some(conflict) = platform.globe_key.conflict() {
        ui.notice(
            Notice::warning(format!(
                "Pressing fn also {conflict}. In System Settings › Keyboard, set \
                 “Press 🌐 key to” to Do Nothing."
            ))
            .with_action(NoticeAction::OpenKeyboardSettings),
        );
    }
    let slot = Arc::new(EngineSlot::new(
        "The speech model is getting ready. Try again in a moment.",
    ));
    let emitter = handle.clone();
    let models = ModelManager::new(
        app.path().app_data_dir()?.join("models"),
        Arc::clone(&slot),
        Box::new(move |statuses| {
            let _ = emitter.emit(MODELS_EVENT, statuses);
        }),
    );

    let controller = dictation::spawn(Services {
        hotkeys: platform.hotkeys,
        bindings: platform.default_bindings,
        push_to_talk_label,
        inserter: platform.inserter,
        permissions: Arc::clone(&platform.permissions),
        sounds: platform.sounds,
        engine: slot,
        stt_options: SttOptions {
            languages: platform.language.into_iter().collect(),
            vocabulary: Vec::new(),
        },
        ui: Arc::clone(&ui) as Arc<dyn DictationUi>,
        source: audio_source(),
        timing: Timing::default(),
    });

    app.manage(controller);
    app.manage(Arc::clone(&models));
    app.manage(ui);
    app.manage(AppPermissions(platform.permissions));
    app.manage(Shortcuts(shortcuts));

    tray::create(&handle)?;
    flow_bar::create(&handle)?;
    tauri::async_runtime::spawn(models.prepare_default());
    Ok(())
}

fn main_thread(handle: &AppHandle) -> MainThread {
    let handle = handle.clone();
    Arc::new(move |work| {
        if let Err(error) = handle.run_on_main_thread(work) {
            log::warn!("could not run on the main thread: {error}");
        }
    })
}

/// The microphone, unless a debug build sets `SHRIMP_AUDIO_SOURCE`.
fn audio_source() -> AudioSourceSpec {
    let microphone = AudioSourceSpec::Microphone { device: None };
    match AudioSourceSpec::from_env() {
        Ok(Some(spec)) => spec,
        Ok(None) => microphone,
        Err(error) => {
            log::warn!("ignoring the audio source override: {error}");
            microphone
        }
    }
}
