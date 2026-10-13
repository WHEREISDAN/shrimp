//! Builds the app's services once at startup and hands them to Tauri's
//! managed state.

use std::sync::Arc;

use tauri::{App, AppHandle, Emitter, Manager};

use super::events::{MODELS_EVENT, SETTINGS_EVENT, TauriUi};
use super::{flow_bar, tray};
use crate::audio::AudioSourceSpec;
use crate::commands::ai::HttpClient;
use crate::commands::permissions::AppPermissions;
use crate::commands::shortcuts::DefaultBindings;
use crate::dictation::{self, ControllerInput, DictationUi, Services, Timing};
use crate::edit::{EditTimeout, Editor, Library};
use crate::llm::{Providers, http};
use crate::notify::{Notice, NoticeAction};
use crate::platform::{self, MainThread};
use crate::secrets::{Keychain, Secrets};
use crate::settings::SettingsStore;
use crate::storage::Database;
use crate::stt::{EngineSlot, ModelManager, SttEngine, SttRouter};

pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    let platform = platform::native(main_thread(&handle));

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
    let data_dir = app.path().app_data_dir()?;
    let settings = Arc::new(SettingsStore::load(data_dir.join("settings.json")));
    let emitter = handle.clone();
    settings.on_change(Box::new(move |settings| {
        let _ = emitter.emit(SETTINGS_EVENT, settings);
    }));
    let secrets = Arc::new(Secrets::new(Box::new(Keychain::new(
        app.config().identifier.clone(),
    ))));
    let database = Arc::new(open_database(&data_dir, ui.as_ref()));
    let client = http::client();
    let editor = Arc::new(Editor::new(
        Arc::clone(&settings),
        Arc::new(Providers::new(client.clone(), Arc::clone(&secrets))),
        Arc::clone(&database) as Arc<dyn Library>,
        EditTimeout::default(),
    ));

    let slot = Arc::new(EngineSlot::new(
        "The speech model is getting ready. Try again in a moment.",
    ));
    let router = SttRouter::new(
        Arc::clone(&slot),
        Arc::clone(&settings),
        Arc::clone(&secrets),
        client.clone(),
    );
    let emitter = handle.clone();
    let models = ModelManager::new(
        data_dir.join("models"),
        Arc::clone(&slot),
        Box::new(move |statuses| {
            let _ = emitter.emit(MODELS_EVENT, statuses);
        }),
    );

    #[cfg(debug_assertions)]
    app.manage(crate::commands::dev::DevFocus(Arc::clone(&platform.focus)));
    app.manage(DefaultBindings {
        bindings: platform.default_bindings.clone(),
        label: platform.label,
    });
    let controller = dictation::spawn(Services {
        hotkeys: platform.hotkeys,
        bindings: platform.default_bindings,
        label: platform.label,
        inserter: platform.inserter,
        focus: platform.focus,
        permissions: Arc::clone(&platform.permissions),
        sounds: platform.sounds,
        launcher: platform.launcher,
        settings: Arc::clone(&settings),
        engine: Arc::new(router),
        fallback_engine: Some(Arc::clone(&slot) as Arc<dyn SttEngine>),
        languages: platform.language.into_iter().collect(),
        editor: Arc::clone(&editor),
        ui: Arc::clone(&ui) as Arc<dyn DictationUi>,
        source: audio_source(),
        timing: Timing::default(),
    });

    let shortcuts_changed = controller.clone();
    settings.on_change(Box::new(move |_| {
        shortcuts_changed.send(ControllerInput::SettingsChanged);
    }));
    app.manage(controller);
    app.manage(Arc::clone(&settings));
    app.manage(secrets);
    app.manage(database);
    app.manage(editor);
    app.manage(HttpClient(client));
    app.manage(Arc::clone(&models));
    app.manage(Arc::clone(&slot));
    app.manage(ui);
    app.manage(AppPermissions(platform.permissions));

    tray::create(&handle)?;
    flow_bar::create(&handle)?;
    tauri::async_runtime::spawn(Arc::clone(&models).prepare(settings.get().stt.local_model));
    let chosen_model = Arc::clone(&models);
    settings.on_change(Box::new(move |settings| {
        // Loads the new choice; the current model serves dictations until then.
        let id = settings.stt.local_model.clone();
        tauri::async_runtime::spawn(Arc::clone(&chosen_model).prepare(id));
    }));
    Ok(())
}

/// The dictionary and snippets database; an unreadable file is set aside for
/// this run (in memory instead) with a notice, so dictation keeps working.
fn open_database(dir: &std::path::Path, ui: &TauriUi) -> Database {
    let path = dir.join("shrimp.db");
    Database::open(&path).unwrap_or_else(|error| {
        log::error!("could not open {}: {error}", path.display());
        ui.notice(Notice::error(
            "Shrimp couldn't open its dictionary and snippets. Changes this session won't be saved.",
        ));
        Database::open_in_memory().expect("an in-memory database always opens")
    })
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
