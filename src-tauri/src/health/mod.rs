//! Error screens (APP-09): a previous run that crashed, a damaged database
//! and a full disk, each with a way out. Reset & restart (APP-08) and the
//! database repairs run at the next launch, before anything opens the files.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::storage::backup;

/// Below this much free space, Shrimp shows the storage screen.
pub const LOW_SPACE_BYTES: u64 = 200 * 1024 * 1024;
const RUNNING_MARKER: &str = "running";
const PANIC_FILE: &str = "last-panic.txt";
const RESET_MARKER: &str = "reset-requested";
const REPAIR_MARKER: &str = "repair-requested";
const DATABASE: &str = "shrimp.db";
/// What Reset & restart keeps: the downloaded speech models.
const KEPT_ON_RESET: [&str; 1] = ["models"];

/// Mirrors `AppHealth` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppHealth {
    /// The previous run ended without quitting.
    pub crash: Option<CrashReport>,
    /// The database couldn't be opened or failed its check.
    pub data: Option<DataProblem>,
    pub storage: Option<StorageProblem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    /// The panic message, when Shrimp itself panicked.
    pub details: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataProblem {
    /// The newest daily backup's date, YYYY-MM-DD.
    pub backup: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageProblem {
    pub free_bytes: u64,
}

/// How to repair a damaged database at the next launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Repair {
    /// Put the newest backup in its place.
    RestoreBackup,
    /// Set it aside and start an empty one.
    StartFresh,
}

pub struct Health {
    dir: PathBuf,
    state: Mutex<AppHealth>,
}

impl Health {
    /// Notes whether the previous run crashed and marks this one as running.
    pub fn start(dir: &Path) -> Self {
        let marker = dir.join(RUNNING_MARKER);
        let crash = marker.exists().then(|| CrashReport {
            details: std::fs::read_to_string(dir.join(PANIC_FILE))
                .ok()
                .map(|text| text.trim().to_owned())
                .filter(|text| !text.is_empty()),
        });
        if crash.is_some() {
            log::warn!("the previous run ended without quitting");
        }
        let _ = std::fs::create_dir_all(dir);
        if let Err(error) = std::fs::write(&marker, std::process::id().to_string()) {
            log::warn!("could not mark the run: {error}");
        }
        Self {
            dir: dir.to_path_buf(),
            state: Mutex::new(AppHealth {
                crash,
                ..AppHealth::default()
            }),
        }
    }

    /// Shrimp is quitting normally.
    pub fn clean_exit(&self) {
        let _ = std::fs::remove_file(self.dir.join(RUNNING_MARKER));
    }

    pub fn get(&self) -> AppHealth {
        self.lock().clone()
    }

    /// Whether anything needs the main window at launch.
    pub fn needs_attention(&self) -> bool {
        let health = self.lock();
        health.crash.is_some() || health.data.is_some() || health.storage.is_some()
    }

    pub fn dismiss_crash(&self) {
        self.lock().crash = None;
        let _ = std::fs::remove_file(self.dir.join(PANIC_FILE));
    }

    pub fn data_damaged(&self) {
        let backup = backup::copies(&self.dir.join("backups"))
            .first()
            .and_then(|path| backup_date(path));
        self.lock().data = Some(DataProblem { backup });
    }

    /// Re-reads the free space; returns whether the storage problem changed.
    pub fn check_storage(&self, free_bytes: Option<u64>) -> bool {
        let problem = free_bytes
            .filter(|free| *free < LOW_SPACE_BYTES)
            .map(|free_bytes| StorageProblem { free_bytes });
        let mut health = self.lock();
        let changed = health.storage.is_some() != problem.is_some();
        health.storage = problem;
        changed
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, AppHealth> {
        self.state.lock().unwrap_or_else(|err| err.into_inner())
    }
}

fn backup_date(path: &Path) -> Option<String> {
    let name = path.file_stem()?.to_str()?;
    name.strip_prefix("shrimp-").map(str::to_owned)
}

/// Writes a panic's message where the next launch finds it.
pub fn record_panics(dir: &Path) {
    let file = dir.join(PANIC_FILE);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = std::fs::write(&file, info.to_string());
        previous(info);
    }));
}

/// Reset & restart (APP-08): everything but the models goes at next launch.
pub fn request_reset(dir: &Path) -> std::io::Result<()> {
    std::fs::write(dir.join(RESET_MARKER), "")
}

pub fn request_repair(dir: &Path, repair: Repair) -> std::io::Result<()> {
    let text = serde_json::to_string(&repair).map_err(std::io::Error::other)?;
    std::fs::write(dir.join(REPAIR_MARKER), text)
}

/// What a pending reset or repair did, run before anything opens the files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pending {
    Nothing,
    Reset,
    Repaired(Repair),
}

pub fn apply_pending(dir: &Path) -> Pending {
    if dir.join(RESET_MARKER).exists() {
        wipe(dir);
        return Pending::Reset;
    }
    let Ok(text) = std::fs::read_to_string(dir.join(REPAIR_MARKER)) else {
        return Pending::Nothing;
    };
    let _ = std::fs::remove_file(dir.join(REPAIR_MARKER));
    let Ok(repair) = serde_json::from_str::<Repair>(&text) else {
        return Pending::Nothing;
    };
    set_aside_database(dir);
    if repair == Repair::RestoreBackup
        && let Some(copy) = backup::copies(&dir.join("backups")).first()
        && let Err(error) = std::fs::copy(copy, dir.join(DATABASE))
    {
        log::error!("could not restore the backup: {error}");
    }
    Pending::Repaired(repair)
}

/// Deletes everything in `dir` except the downloaded models.
fn wipe(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        if KEPT_ON_RESET.iter().any(|kept| name == *kept) {
            continue;
        }
        let path = entry.path();
        let removed = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        if let Err(error) = removed {
            log::warn!("could not delete {}: {error}", path.display());
        }
    }
}

/// Renames the damaged database (and its WAL files) out of the way.
fn set_aside_database(dir: &Path) {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    for suffix in ["", "-wal", "-shm"] {
        let path = dir.join(format!("{DATABASE}{suffix}"));
        if path.exists() {
            let aside = dir.join(format!("{DATABASE}{suffix}.damaged-{stamp}"));
            if let Err(error) = std::fs::rename(&path, &aside) {
                log::warn!("could not set {} aside: {error}", path.display());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_that_never_quit_is_a_crash() {
        let dir = tempfile::tempdir().unwrap();
        let first = Health::start(dir.path());
        assert_eq!(first.get().crash, None);
        first.clean_exit();
        assert_eq!(Health::start(dir.path()).get().crash, None);
        // That run never quit.
        std::fs::write(dir.path().join(PANIC_FILE), "panicked at src/x.rs:1").unwrap();
        let after_crash = Health::start(dir.path());
        assert_eq!(
            after_crash.get().crash,
            Some(CrashReport {
                details: Some("panicked at src/x.rs:1".into())
            })
        );
        assert!(after_crash.needs_attention());
        after_crash.dismiss_crash();
        assert!(!after_crash.needs_attention());
    }

    #[test]
    fn low_space_comes_and_goes() {
        let dir = tempfile::tempdir().unwrap();
        let health = Health::start(dir.path());
        assert!(health.check_storage(Some(50 * 1024 * 1024)));
        assert_eq!(health.get().storage.unwrap().free_bytes, 50 * 1024 * 1024);
        assert!(!health.check_storage(Some(10)));
        assert!(health.check_storage(Some(LOW_SPACE_BYTES * 4)));
        assert_eq!(health.get().storage, None);
    }

    #[test]
    fn reset_keeps_only_the_models() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("models")).unwrap();
        std::fs::write(dir.path().join("models/ggml.bin"), "model").unwrap();
        std::fs::create_dir_all(dir.path().join("recordings")).unwrap();
        std::fs::write(dir.path().join("recordings/1.wav"), "wav").unwrap();
        std::fs::write(dir.path().join("settings.json"), "{}").unwrap();
        std::fs::write(dir.path().join(DATABASE), "db").unwrap();
        request_reset(dir.path()).unwrap();
        assert_eq!(apply_pending(dir.path()), Pending::Reset);
        let left: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(left, ["models"]);
        assert!(dir.path().join("models/ggml.bin").exists());
        assert_eq!(apply_pending(dir.path()), Pending::Nothing);
    }

    #[test]
    fn a_damaged_database_is_replaced_by_the_newest_backup() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("backups")).unwrap();
        std::fs::write(dir.path().join("backups/shrimp-2026-10-12.db"), "older").unwrap();
        std::fs::write(dir.path().join("backups/shrimp-2026-10-13.db"), "newest").unwrap();
        std::fs::write(dir.path().join(DATABASE), "garbage").unwrap();
        let health = Health::start(dir.path());
        health.data_damaged();
        assert_eq!(
            health.get().data.unwrap().backup.as_deref(),
            Some("2026-10-13")
        );
        request_repair(dir.path(), Repair::RestoreBackup).unwrap();
        assert_eq!(
            apply_pending(dir.path()),
            Pending::Repaired(Repair::RestoreBackup)
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join(DATABASE)).unwrap(),
            "newest"
        );
        let aside = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("shrimp.db.damaged-")
            });
        assert!(aside, "the damaged file is kept aside");
    }
}
