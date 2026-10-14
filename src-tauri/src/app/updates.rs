//! Updates (PKG-03, PKG-04): the tray's "Check for updates…", and a check
//! soon after launch and every six hours, look for a newer release on the
//! feed set at build time (`plugins.updater.endpoints`, see
//! docs/RELEASING.md). A release is downloaded and its signature checked
//! before "Update ready, restart now?" is offered. Installing waits until no
//! dictation is running, then Shrimp restarts; the first launch of the new
//! version opens the main window (`app::setup`).

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tauri::menu::MenuItem;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_updater::{Update, Updater, UpdaterExt};

use super::tray;
use crate::dictation::{ControllerHandle, DictationState};
use crate::notify::{Notice, NoticeAction};
use crate::settings::SettingsStore;
use crate::t;

pub const MENU_ID: &str = "check-updates";
const FIRST_CHECK: Duration = Duration::from_secs(60);
const AUTO_CHECK: Duration = Duration::from_secs(6 * 60 * 60);
const IDLE_POLL: Duration = Duration::from_millis(250);
/// Idle this long before installing, so a paste's clipboard restore can finish.
const SETTLE: Duration = Duration::from_secs(1);
/// The same after a dictation ended while waiting: longer than the remote
/// desktop clipboard hold (INS-10).
const SETTLE_AFTER_DICTATION: Duration = Duration::from_secs(6);

pub type Notify = Arc<dyn Fn(Notice) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// No feed was set at build time.
    Unconfigured,
    Idle,
    Checking,
    Downloading,
    Ready,
    /// Waiting for dictation to finish, then installing.
    Restarting,
}

struct Ready {
    update: Update,
    bytes: Vec<u8>,
}

struct Inner {
    status: Status,
    ready: Option<Ready>,
}

pub struct Updates {
    controller: ControllerHandle,
    notify: Notify,
    inner: Mutex<Inner>,
}

impl Updates {
    pub fn new<R: Runtime>(
        app: &AppHandle<R>,
        controller: ControllerHandle,
        notify: Notify,
    ) -> Self {
        let status = if feed_configured(app) {
            Status::Idle
        } else {
            Status::Unconfigured
        };
        Self {
            controller,
            notify,
            inner: Mutex::new(Inner {
                status,
                ready: None,
            }),
        }
    }

    pub fn status(&self) -> Status {
        self.lock().status
    }

    /// Moves from `from` to `to`; false when the status was something else.
    fn advance(&self, from: Status, to: Status) -> bool {
        let mut inner = self.lock();
        let moved = inner.status == from;
        if moved {
            inner.status = to;
        }
        moved
    }

    fn set(&self, status: Status) {
        self.lock().status = status;
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|err| err.into_inner())
    }
}

/// Whether the build carries an update feed.
fn feed_configured<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|updater| updater.get("endpoints"))
        .and_then(serde_json::Value::as_array)
        .is_some_and(|endpoints| !endpoints.is_empty())
}

/// Checks soon after launch and then every six hours, quietly unless a
/// release is ready.
pub fn start<R: Runtime>(app: &AppHandle<R>) {
    if state(app).is_none_or(|updates| updates.status() == Status::Unconfigured) {
        log::info!("updates aren't configured in this build");
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            check(&app, false);
            tokio::time::sleep(AUTO_CHECK).await;
        }
    });
}

/// The tray item: what it says follows the status.
pub fn menu_item<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<MenuItem<R>> {
    let status = state(app).map_or(Status::Unconfigured, |updates| updates.status());
    let (text, enabled) = match status {
        Status::Unconfigured => (t!("tray.updatesNotConfigured"), false),
        Status::Idle => (t!("tray.checkForUpdates"), true),
        Status::Checking => (t!("tray.checkingForUpdates"), false),
        Status::Downloading => (t!("tray.downloadingUpdate"), false),
        Status::Ready => (t!("tray.restartToUpdate"), true),
        Status::Restarting => (t!("tray.restartingToUpdate"), false),
    };
    MenuItem::with_id(app, MENU_ID, text, enabled, None::<&str>)
}

pub fn menu_clicked<R: Runtime>(app: &AppHandle<R>) {
    match state(app).map(|updates| updates.status()) {
        Some(Status::Ready) => restart_when_idle(app),
        Some(Status::Idle) => check(app, true),
        _ => {}
    }
}

/// Looks for a newer release and downloads it. A `manual` check also says
/// when Shrimp is up to date or the check failed.
pub fn check<R: Runtime>(app: &AppHandle<R>, manual: bool) {
    let Some(updates) = state(app) else {
        return;
    };
    if !updates.advance(Status::Idle, Status::Checking) {
        if manual && updates.status() == Status::Ready {
            offer_restart(&updates);
        }
        return;
    }
    refresh_tray(app);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let found = match updater(&app) {
            Ok(updater) => updater.check().await,
            Err(error) => Err(error),
        };
        match found {
            Ok(Some(update)) => download(&app, &updates, update, manual).await,
            Ok(None) => {
                log::info!("no update found");
                updates.set(Status::Idle);
                if manual {
                    let version = app.package_info().version.to_string();
                    (updates.notify)(Notice::info(t!(
                        "notice.update.upToDate",
                        version = version
                    )));
                }
            }
            Err(error) => {
                log::warn!("could not check for updates: {error}");
                updates.set(Status::Idle);
                if manual {
                    (updates.notify)(Notice::error(t!(
                        "notice.update.checkFailed",
                        error = error
                    )));
                }
            }
        }
        refresh_tray(&app);
    });
}

/// On Windows, installing starts the NSIS installer and the plugin ends the
/// process itself (`std::process::exit`), so `RunEvent::Exit` never runs and
/// its work happens here instead. The installer then starts the new version.
fn updater<R: Runtime>(app: &AppHandle<R>) -> tauri_plugin_updater::Result<Updater> {
    let handle = app.clone();
    app.updater_builder()
        .on_before_exit(move || {
            handle.cleanup_before_exit();
            super::before_exit(&handle);
        })
        .build()
}

async fn download<R: Runtime>(app: &AppHandle<R>, updates: &Updates, update: Update, manual: bool) {
    log::info!("downloading Shrimp {}", update.version);
    updates.set(Status::Downloading);
    refresh_tray(app);
    if manual {
        let version = update.version.clone();
        (updates.notify)(Notice::info(t!(
            "notice.update.downloading",
            version = version
        )));
    }
    // The signature is checked against the public key in the configuration.
    match update.download(|_, _| {}, || {}).await {
        Ok(bytes) => {
            {
                let mut inner = updates.lock();
                inner.status = Status::Ready;
                inner.ready = Some(Ready { update, bytes });
            }
            offer_restart(updates);
        }
        Err(error) => {
            log::warn!("could not download the update: {error}");
            updates.set(Status::Idle);
            if manual {
                (updates.notify)(Notice::error(t!(
                    "notice.update.downloadFailed",
                    error = error
                )));
            }
        }
    }
}

fn offer_restart(updates: &Updates) {
    let version = updates
        .lock()
        .ready
        .as_ref()
        .map(|ready| ready.update.version.clone())
        .unwrap_or_default();
    (updates.notify)(
        Notice::info(t!("notice.update.ready", version = version))
            .with_action(NoticeAction::RestartToUpdate),
    );
}

/// Installs the downloaded release once no dictation is running, then
/// restarts into it.
pub fn restart_when_idle<R: Runtime>(app: &AppHandle<R>) {
    let Some(updates) = state(app) else {
        return;
    };
    if !updates.advance(Status::Ready, Status::Restarting) {
        return;
    }
    refresh_tray(app);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut settle = Settle::default();
        let mut told = false;
        while !settle.observe(
            updates.controller.state() == DictationState::Idle,
            Instant::now(),
        ) {
            if settle.saw_dictation && !told {
                told = true;
                (updates.notify)(Notice::info(t!("notice.update.waiting")));
            }
            tokio::time::sleep(IDLE_POLL).await;
        }
        let Some(ready) = updates.lock().ready.take() else {
            return;
        };
        log::info!("installing Shrimp {}", ready.update.version);
        let installed =
            tauri::async_runtime::spawn_blocking(move || ready.update.install(&ready.bytes)).await;
        let error = match installed {
            Ok(Ok(())) => match crate::commands::system::restart(&app) {
                Ok(()) => return,
                Err(error) => error.message,
            },
            Ok(Err(error)) => error.to_string(),
            Err(error) => error.to_string(),
        };
        log::error!("could not install the update: {error}");
        updates.set(Status::Idle);
        (updates.notify)(Notice::error(t!(
            "notice.update.installFailed",
            error = error
        )));
        refresh_tray(&app);
    });
}

/// Decides when installing can start: no dictation, and a quiet moment
/// after the last one.
#[derive(Debug, Default)]
struct Settle {
    idle_since: Option<Instant>,
    saw_dictation: bool,
}

impl Settle {
    /// Records one look at the dictation state; true once it is safe.
    fn observe(&mut self, idle: bool, now: Instant) -> bool {
        if !idle {
            self.saw_dictation = true;
            self.idle_since = None;
            return false;
        }
        let since = *self.idle_since.get_or_insert(now);
        let needed = if self.saw_dictation {
            SETTLE_AFTER_DICTATION
        } else {
            SETTLE
        };
        now.duration_since(since) >= needed
    }
}

fn state<R: Runtime>(app: &AppHandle<R>) -> Option<Arc<Updates>> {
    app.try_state::<Arc<Updates>>()
        .map(|updates| Arc::clone(&updates))
}

fn refresh_tray<R: Runtime>(app: &AppHandle<R>) {
    let Some(settings) = app.try_state::<Arc<SettingsStore>>() else {
        return;
    };
    let settings = settings.get();
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || tray::refresh(&handle, &settings));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_after_a_short_quiet_moment_when_nothing_was_dictated() {
        let start = Instant::now();
        let mut settle = Settle::default();
        assert!(!settle.observe(true, start));
        assert!(!settle.observe(true, start + SETTLE / 2));
        assert!(settle.observe(true, start + SETTLE));
    }

    #[test]
    fn waits_for_a_running_dictation_and_its_clipboard_restore() {
        let start = Instant::now();
        let mut settle = Settle::default();
        assert!(!settle.observe(false, start));
        assert!(!settle.observe(false, start + Duration::from_secs(30)));
        let ended = start + Duration::from_secs(31);
        assert!(!settle.observe(true, ended));
        assert!(!settle.observe(true, ended + SETTLE));
        assert!(settle.saw_dictation);
        assert!(settle.observe(true, ended + SETTLE_AFTER_DICTATION));
    }

    #[test]
    fn a_new_dictation_starts_the_wait_again() {
        let start = Instant::now();
        let mut settle = Settle::default();
        assert!(!settle.observe(true, start));
        assert!(!settle.observe(false, start + SETTLE / 2));
        let ended = start + SETTLE;
        assert!(!settle.observe(true, ended));
        assert!(!settle.observe(true, ended + SETTLE));
        assert!(settle.observe(true, ended + SETTLE_AFTER_DICTATION));
    }
}
