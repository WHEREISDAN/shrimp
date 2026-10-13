//! Builds the app's services once at startup and hands them to Tauri's
//! managed state.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{App, AppHandle, Emitter, Manager};

use super::events::{HISTORY_EVENT, MICROPHONES_EVENT, MODELS_EVENT, SETTINGS_EVENT, TauriUi};
use super::updates::{self, Updates};
use super::{flow_bar, system, tray, window};
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
use crate::health::Pending;
use crate::history::History;
use crate::i18n;
use crate::lang;
use crate::llm::{LlmSource, Providers, http};
use crate::mic::Microphones;
use crate::notify::{Notice, NoticeAction};
#[cfg(debug_assertions)]
use crate::platform::SwitchablePermissions;
use crate::platform::{self, CuePlayer, MainThread, Permissions};
use crate::secrets::{Keychain, Secrets};
use crate::settings::{Settings, SettingsStore};
use crate::sounds::{SettingsCues, SoundingNotices};
use crate::storage::Database;
use crate::stt::{EngineSlot, ModelManager, SttEngine, SttRouter};
use crate::t;

pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    // `--quit` reached no running instance: nothing to quit.
    if cfg!(debug_assertions) && std::env::args().any(|arg| arg == "--quit") {
        std::process::exit(0);
    }
    let handle = app.handle().clone();
    let platform = platform::native(main_thread(&handle));

    let data_dir = app.path().app_data_dir()?;
    let (health, pending) = system::prepare(&data_dir);
    let settings = Arc::new(SettingsStore::load(data_dir.join("settings.json")));
    follow_interface_language(&settings, platform.language.clone());
    let ui = Arc::new(TauriUi::new(handle.clone()));
    if let Some(conflict) = platform.globe_key.conflict() {
        ui.notice(
            Notice::warning(t!("notice.setup.globeKey", conflict = conflict))
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
    if pending == Pending::Reset {
        system::forget_keys(&secrets);
    }
    let database = Arc::new(system::open_database(&data_dir, &health));
    system::watch_storage(&handle, &health, &data_dir);
    let history = start_history(&handle, &database, &data_dir, &settings, ui.as_ref());
    let client = http::client();
    let llm: Arc<dyn LlmSource> = Arc::new(Providers::new(client.clone(), Arc::clone(&secrets)));
    let editor = Arc::new(Editor::new(
        Arc::clone(&settings),
        Arc::clone(&llm),
        Arc::clone(&database) as Arc<dyn Library>,
        EditTimeout::default(),
    ));

    let slot = Arc::new(EngineSlot::new(t!("notice.setup.modelGettingReady")));
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
    let devices: Arc<dyn DeviceProvider> = platform.devices;
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
    let notices = Arc::clone(&dictation_ui);
    let update_notices = Arc::clone(&dictation_ui);
    let login_item = Arc::clone(&platform.login_item);
    #[cfg(debug_assertions)]
    let permissions: Arc<dyn Permissions> = {
        let switchable = Arc::new(SwitchablePermissions::new(platform.permissions));
        switchable.pretend_from_env();
        app.manage(Arc::clone(&switchable));
        switchable
    };
    #[cfg(not(debug_assertions))]
    let permissions: Arc<dyn Permissions> = platform.permissions;
    let version = app.package_info().version.to_string();
    // The first launch of a new version opens the main window, as after an update (PKG-04).
    let updated =
        settings.get().onboarding.completed && settings.get().onboarding.version != version;
    system::recheck_permissions(
        &settings,
        permissions.as_ref(),
        &version,
        dictation_ui.as_ref(),
    );
    let controller = dictation::spawn(Services {
        hotkeys: platform.hotkeys,
        bindings: platform.default_bindings,
        label: platform.label,
        reserved: platform.reserved,
        inserter: platform.inserter,
        focus: platform.focus,
        permissions: Arc::clone(&permissions),
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
    app.manage(controller.clone());
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
    app.manage(AppPermissions(permissions));
    app.manage(Arc::clone(&health));
    app.manage(Arc::new(Updates::new(
        &handle,
        controller,
        Arc::new(move |notice| update_notices.notice(notice)),
    )));

    tray::create(&handle, &settings.get())?;
    let tray_handle = handle.clone();
    let shown =
        |settings: &Settings| (settings.privacy.privacy_mode, settings.app.language.clone());
    let previous = Mutex::new(shown(&settings.get()));
    settings.on_change(Box::new(move |settings| {
        let mut previous = previous.lock().unwrap_or_else(|err| err.into_inner());
        let next = shown(settings);
        if *previous == next {
            return;
        }
        let language_changed = previous.1 != next.1;
        *previous = next;
        let tray_handle = tray_handle.clone();
        let settings = settings.clone();
        let _ = tray_handle.clone().run_on_main_thread(move || {
            tray::refresh(&tray_handle, &settings);
            if language_changed {
                flow_bar::retitle(&tray_handle);
            }
        });
    }));
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
    system::follow_system_settings(
        &handle,
        &settings,
        Arc::clone(&login_item),
        Arc::new(move |notice| notices.notice(notice)),
    );
    updates::start(&handle);
    // The main window starts hidden; a launch at login stays in the menu bar (APP-06),
    // unless setup isn't finished or Shrimp was just updated.
    if updated
        || !settings.get().onboarding.completed
        || !system::starts_in_background(login_item.as_ref(), &health)
    {
        window::show_main(&handle);
    }
    Ok(())
}

/// Text from Rust follows Settings › General › App language (LANG-06). This
/// runs first on every change, so the reactions to a change use the new language.
fn follow_interface_language(settings: &SettingsStore, preferred: Option<String>) {
    let apply =
        move |setting: &str| i18n::set_language(i18n::resolve(setting, preferred.as_deref()));
    apply(&settings.get().app.language);
    settings.on_change(Box::new(move |settings| apply(&settings.app.language)));
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
            Notice::info(t!("notice.setup.recovered")).with_action(NoticeAction::OpenHistory),
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
