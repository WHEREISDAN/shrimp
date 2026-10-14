//! Startup chores of the app shell: pending resets and repairs, crash and
//! storage checks (APP-08, APP-09), launch at login and the Dock icon
//! (APP-06, APP-07).

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use tauri::{AppHandle, Emitter};

use super::events::HEALTH_EVENT;
use super::window;
use crate::commands::system::free_bytes;
use crate::dictation::DictationUi;
use crate::health::{self, Health, Pending};
use crate::notify::{Notice, NoticeAction};
use crate::platform::{LoginItem, Permission, PermissionStatus, Permissions, RELAUNCH_ARG};
use crate::secrets::{KeyAccount, Secrets};
use crate::settings::SettingsStore;
use crate::storage::Database;
use crate::t;

/// How often the free disk space is checked.
const STORAGE_CHECK: Duration = Duration::from_secs(10 * 60);
/// The argument a launch at login may carry (and tests use).
const BACKGROUND_ARG: &str = "--background";

/// Runs a reset or repair asked for before the restart, then starts the
/// crash check. Call before anything opens the data files.
pub fn prepare(data_dir: &Path) -> (Arc<Health>, Pending) {
    let pending = health::apply_pending(data_dir);
    match pending {
        Pending::Reset => log::info!("reset the local data"),
        Pending::Repaired(repair) => log::info!("repaired the database ({repair:?})"),
        Pending::Nothing => {}
    }
    health::record_panics(data_dir);
    (Arc::new(Health::start(data_dir)), pending)
}

/// Reset & restart also forgets the API keys.
pub fn forget_keys(secrets: &Secrets) {
    for account in KeyAccount::ALL {
        if let Err(error) = secrets.delete(account) {
            log::warn!("could not delete the {} key: {error}", account.label());
        }
    }
}

/// The database, or an in-memory one for this run when the file is damaged;
/// the damaged-data screen then offers the backup (APP-09).
pub fn open_database(dir: &Path, health: &Health) -> Database {
    let path = dir.join("shrimp.db");
    let opened = Database::open(&path).and_then(|database| {
        database.check()?;
        Ok(database)
    });
    opened.unwrap_or_else(|error| {
        log::error!("could not open {}: {error}", path.display());
        health.data_damaged();
        Database::open_in_memory().expect("an in-memory database always opens")
    })
}

/// Checks the free space now and every ten minutes, telling the webview
/// when the storage screen should come or go.
pub fn watch_storage(handle: &AppHandle, health: &Arc<Health>, data_dir: &Path) {
    health.check_storage(free_bytes(data_dir));
    let handle = handle.clone();
    let health = Arc::clone(health);
    let dir = data_dir.to_path_buf();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(STORAGE_CHECK).await;
            if health.check_storage(free_bytes(&dir)) {
                let _ = handle.emit(HEALTH_EVENT, health.get());
                window::show_main(&handle);
            }
        }
    });
}

/// Keeps Settings › System's switches and macOS in step: the login item's
/// real state wins at startup (the user may turn it off in System
/// Settings), and changes apply as they are made.
pub fn follow_system_settings(
    handle: &AppHandle,
    settings: &Arc<SettingsStore>,
    login_item: Arc<dyn LoginItem>,
    ui: Arc<dyn Fn(Notice) + Send + Sync>,
) {
    let enabled = login_item.enabled();
    if settings.get().app.launch_at_login != enabled
        && let Err(error) = settings.update(&json!({"app": {"launchAtLogin": enabled}}))
    {
        log::warn!("could not note the login item: {error}");
    }
    let app = settings.get().app;
    window::show_in_dock(handle, app.show_in_dock);
    let previous = Mutex::new(app);
    let handle = handle.clone();
    let store = Arc::clone(settings);
    settings.on_change(Box::new(move |settings| {
        let mut previous = previous.lock().unwrap_or_else(|err| err.into_inner());
        let next = settings.app.clone();
        if next.show_in_dock != previous.show_in_dock {
            let handle = handle.clone();
            let show = next.show_in_dock;
            let _ = handle
                .clone()
                .run_on_main_thread(move || window::show_in_dock(&handle, show));
        }
        if next.launch_at_login != previous.launch_at_login
            && login_item.enabled() != next.launch_at_login
            && let Err(error) = login_item.set_enabled(next.launch_at_login)
        {
            log::warn!("could not change the login item: {error}");
            let message = t!("notice.launchAtLoginFailed", error = error);
            ui(Notice::error(message));
            // Put the switch back once this change has finished.
            let store = Arc::clone(&store);
            let actual = login_item.enabled();
            tauri::async_runtime::spawn(async move {
                let _ = store.update(&json!({"app": {"launchAtLogin": actual}}));
            });
        }
        *previous = next;
    }));
}

/// Whether this launch should stay in the background: launched at login
/// (or with `--background`), unless Shrimp restarted itself or something
/// needs the main window.
pub fn starts_in_background(login_item: &dyn LoginItem, health: &Health) -> bool {
    if std::env::args().any(|arg| arg == RELAUNCH_ARG) {
        return false;
    }
    let asked = std::env::args().any(|arg| arg == BACKGROUND_ARG) || login_item.launched_at_login();
    if asked {
        log::info!("started in the background");
    }
    asked && !health.needs_attention()
}

/// After an update, macOS may drop a grant (the signature changed): a
/// permission seen granted before and missing now gets a notice, and the
/// main window asks for it again (ONB-08). Grants seen now are noted.
pub fn recheck_permissions(
    settings: &SettingsStore,
    permissions: &dyn Permissions,
    version: &str,
    ui: &dyn DictationUi,
) {
    let onboarding = settings.get().onboarding;
    let microphone = permissions.status(Permission::Microphone) == PermissionStatus::Granted;
    let accessibility = permissions.status(Permission::Accessibility) == PermissionStatus::Granted;
    let updated = onboarding.version != version;
    if onboarding.completed && updated {
        let lost = [
            (
                onboarding.granted.accessibility && !accessibility,
                Permission::Accessibility,
            ),
            (
                onboarding.granted.microphone && !microphone,
                Permission::Microphone,
            ),
        ];
        for (_, permission) in lost.iter().filter(|(lost, _)| *lost) {
            log::warn!("{permission:?} was lost after the update to {version}");
            ui.notice(lost_permission_notice(*permission));
        }
    }
    let patch = json!({"onboarding": {
        "version": version,
        "granted": {
            "microphone": onboarding.granted.microphone || microphone,
            "accessibility": onboarding.granted.accessibility || accessibility,
        },
    }});
    if let Err(error) = settings.update(&patch) {
        log::warn!("could not note the permissions: {error}");
    }
}

fn lost_permission_notice(permission: Permission) -> Notice {
    match permission {
        Permission::Accessibility => Notice::error(t!("notice.setup.lostAccessibility"))
            .with_action(NoticeAction::OpenAccessibilitySettings),
        Permission::Microphone => Notice::error(t!("notice.setup.lostMicrophone"))
            .with_action(NoticeAction::OpenMicrophoneSettings),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::dictation::DictationState;
    use crate::platform::PlatformError;
    use crate::settings::{OnboardingSettings, Settings};

    struct Statuses(PermissionStatus, PermissionStatus);

    impl Permissions for Statuses {
        fn status(&self, permission: Permission) -> PermissionStatus {
            match permission {
                Permission::Microphone => self.0,
                Permission::Accessibility => self.1,
            }
        }

        fn request(&self, _: Permission) -> Result<(), PlatformError> {
            Ok(())
        }

        fn open_settings(&self, _: Permission) -> Result<(), PlatformError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct Notices(Mutex<Vec<Notice>>);

    impl DictationUi for Notices {
        fn state(&self, _: DictationState) {}
        fn level(&self, _: f32) {}
        fn notice(&self, notice: Notice) {
            self.0.lock().unwrap().push(notice);
        }
    }

    fn installed(version: &str) -> SettingsStore {
        let mut onboarding = OnboardingSettings::done();
        onboarding.version = version.into();
        onboarding.granted.accessibility = true;
        onboarding.granted.microphone = true;
        SettingsStore::in_memory(Settings {
            onboarding,
            ..Settings::default()
        })
    }

    #[test]
    fn a_grant_lost_in_an_update_is_asked_for_again() {
        let settings = installed("0.1.0");
        let notices = Notices::default();
        let permissions = Statuses(PermissionStatus::Granted, PermissionStatus::Denied);
        recheck_permissions(&settings, &permissions, "0.2.0", &notices);
        let shown = notices.0.lock().unwrap();
        assert_eq!(shown.len(), 1);
        assert!(shown[0].message.contains("lost Accessibility access"));
        assert_eq!(settings.get().onboarding.version, "0.2.0");
        assert!(
            settings.get().onboarding.granted.accessibility,
            "still owed"
        );
    }

    #[test]
    fn the_same_version_and_new_grants_raise_nothing() {
        let settings = installed("0.2.0");
        let notices = Notices::default();
        let permissions = Statuses(PermissionStatus::Granted, PermissionStatus::Denied);
        recheck_permissions(&settings, &permissions, "0.2.0", &notices);
        assert!(notices.0.lock().unwrap().is_empty());

        let fresh = SettingsStore::in_memory(Settings::default());
        let granted = Statuses(PermissionStatus::Granted, PermissionStatus::Granted);
        recheck_permissions(&fresh, &granted, "0.2.0", &notices);
        assert!(notices.0.lock().unwrap().is_empty());
        assert!(fresh.get().onboarding.granted.microphone);
    }
}
