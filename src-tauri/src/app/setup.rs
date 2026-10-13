//! Builds the app's services once at startup and hands them to Tauri's
//! managed state.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{App, AppHandle, Emitter, Manager};

use super::events::{HISTORY_EVENT, MICROPHONES_EVENT, MODELS_EVENT, SETTINGS_EVENT, TauriUi};
use super::{flow_bar, tray};
#[cfg(debug_assertions)]
use crate::audio::SwitchableDevices;
use crate::audio::{AudioSourceSpec, DeviceProvider};
use crate::commands::ai::HttpClient;
use crate::commands::app::UserName;
use crate::commands::history::Clipboard;
use crate::commands::insights::AppLlm;
use crate::commands::permissions::AppPermissions;
use crate::commands::shortcuts::DefaultBindings;
use crate::dictation::{self, ControllerInput, DictationUi, MutedNotices, Services, Timing};
use crate::edit::{EditTimeout, Editor, Library};
use crate::history::History;
use crate::lang;
use crate::llm::{LlmSource, Providers, http};
use crate::mic::Microphones;
use crate::notify::{Notice, NoticeAction};
use crate::platform::{self, CuePlayer, MainThread};
use crate::secrets::{Keychain, Secrets};
use crate::settings::SettingsStore;
use crate::sounds::{SettingsCues, SoundingNotices};
use crate::storage::Database;
use crate::stt::{EngineSlot, ModelManager, SttEngine, SttRouter};

pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    let platform = platform::native(main_thread(&handle));

    let data_dir = app.path().app_data_dir()?;
    let settings = Arc::new(SettingsStore::load(data_dir.join("settings.json")));
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
    let emitter = handle.clone();
    settings.on_change(Box::new(move |settings| {
        let _ = emitter.emit(SETTINGS_EVENT, settings);
    }));
    first_languages(&settings, platform.language.as_deref());
    let secrets = Arc::new(Secrets::new(Box::new(Keychain::new(
        app.config().identifier.clone(),
    ))));
    let database = Arc::new(open_database(&data_dir, ui.as_ref()));
    let history = start_history(&handle, &database, &data_dir, &settings, ui.as_ref());
    let client = http::client();
    let llm: Arc<dyn LlmSource> = Arc::new(Providers::new(client.clone(), Arc::clone(&secrets)));
    let editor = Arc::new(Editor::new(
        Arc::clone(&settings),
        Arc::clone(&llm),
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
    // Sounds follow Settings › Sound, and warnings come with the notice sound (SND-02).
    let cues: Arc<dyn CuePlayer> =
        Arc::new(SettingsCues::new(platform.sounds, Arc::clone(&settings)));
    // Muted categories are dropped before they can sound (NOT-01).
    let dictation_ui: Arc<dyn DictationUi> = Arc::new(MutedNotices::new(
        Arc::new(SoundingNotices::new(
            Arc::clone(&ui) as Arc<dyn DictationUi>,
            Arc::clone(&cues),
        )),
        Arc::clone(&settings),
    ));
    #[cfg(debug_assertions)]
    let devices: Arc<dyn DeviceProvider> = {
        let switchable = Arc::new(SwitchableDevices::new(platform.devices));
        app.manage(Arc::clone(&switchable));
        switchable
    };
    #[cfg(not(debug_assertions))]
    let devices = platform.devices;
    let emitter = handle.clone();
    let microphones = Microphones::new(
        devices,
        Arc::clone(&settings),
        Arc::clone(&dictation_ui),
        Box::new(move |list| {
            let _ = emitter.emit(MICROPHONES_EVENT, list);
        }),
    );
    Arc::clone(&microphones).watch();
    app.manage(Clipboard(Arc::clone(&platform.inserter)));
    app.manage(UserName(platform.user_name.clone()));
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
        sounds: cues,
        launcher: platform.launcher,
        media: platform.media,
        settings: Arc::clone(&settings),
        engine: Arc::new(router),
        fallback_engine: Some(Arc::clone(&slot) as Arc<dyn SttEngine>),
        editor: Arc::clone(&editor),
        ui: dictation_ui,
        source: audio_source(),
        microphones: Some(Arc::clone(&microphones)),
        history: Some(Arc::clone(&history)),
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
    app.manage(history);
    app.manage(editor);
    app.manage(HttpClient(client));
    app.manage(AppLlm(llm));
    app.manage(Arc::clone(&models));
    app.manage(microphones);
    app.manage(Arc::clone(&slot));
    app.manage(ui);
    app.manage(AppPermissions(platform.permissions));

    tray::create(&handle)?;
    flow_bar::create(&handle, Arc::clone(&settings))?;
    let placement = handle.clone();
    settings.on_change(Box::new(move |_| flow_bar::settings_changed(&placement)));
    tauri::async_runtime::spawn(Arc::clone(&models).prepare(settings.get().stt.local_model));
    let chosen_model = Arc::clone(&models);
    settings.on_change(Box::new(move |settings| {
        // Loads the new choice; the current model serves dictations until then.
        let id = settings.stt.local_model.clone();
        tauri::async_runtime::spawn(Arc::clone(&chosen_model).prepare(id));
    }));
    Ok(())
}

/// On first launch the Mac's language becomes the language to dictate in.
fn first_languages(settings: &SettingsStore, locale: Option<&str>) {
    let languages = settings.get().languages;
    if languages.auto_detect || !languages.selected.is_empty() {
        return;
    }
    let id = locale
        .and_then(lang::catalog::from_locale)
        .unwrap_or_else(|| "en-US".to_owned());
    if let Err(error) = settings.update(&serde_json::json!({"languages": {"selected": [id]}})) {
        log::warn!("could not set the first language: {error}");
    }
}

/// How often History deletes expired recordings and entries and backs up
/// the database (HIS-04, HIS-07, HIS-08).
const HISTORY_SWEEP: Duration = Duration::from_secs(60 * 60);

/// History, with what a quit or crash left behind recovered, its hourly
/// sweep, and Local data storage applied when it changes.
fn start_history(
    handle: &AppHandle,
    database: &Arc<Database>,
    data_dir: &std::path::Path,
    settings: &Arc<SettingsStore>,
    ui: &TauriUi,
) -> Arc<History> {
    let emitter = handle.clone();
    let history = History::new(
        Arc::clone(database),
        data_dir,
        Arc::clone(settings),
        Box::new(move || {
            let _ = emitter.emit(HISTORY_EVENT, ());
        }),
    );
    if history.recover_interrupted() > 0 {
        ui.notice(
            Notice::info("Shrimp saved a dictation that was interrupted. Recover it in History.")
                .with_action(NoticeAction::OpenHistory),
        );
    }
    let sweeper = Arc::clone(&history);
    tauri::async_runtime::spawn(async move {
        loop {
            let history = Arc::clone(&sweeper);
            let _ = tauri::async_runtime::spawn_blocking(move || history.sweep()).await;
            tokio::time::sleep(HISTORY_SWEEP).await;
        }
    });
    let storage = Arc::clone(&history);
    let previous = Mutex::new(settings.get().privacy.history);
    settings.on_change(Box::new(move |settings| {
        let mut previous = previous.lock().unwrap_or_else(|err| err.into_inner());
        if *previous != settings.privacy.history {
            *previous = settings.privacy.history;
            storage.apply_storage();
        }
    }));
    history
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
