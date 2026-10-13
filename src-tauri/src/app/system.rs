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
use crate::health::{self, Health, Pending};
use crate::notify::Notice;
use crate::platform::LoginItem;
use crate::secrets::{KeyAccount, Secrets};
use crate::settings::SettingsStore;
use crate::storage::Database;

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
            ui(Notice::error(format!(
                "Couldn't change Launch at login: {error}"
            )));
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
/// (or with `--background`), unless something needs the main window.
pub fn starts_in_background(login_item: &dyn LoginItem, health: &Health) -> bool {
    let asked = std::env::args().any(|arg| arg == BACKGROUND_ARG) || login_item.launched_at_login();
    if asked {
        log::info!("started in the background");
    }
    asked && !health.needs_attention()
}
