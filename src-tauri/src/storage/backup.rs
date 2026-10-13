//! A copy of the database once a day (HIS-08), kept for recovery when the
//! database is damaged (APP-09).

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::db::StorageError;

const PREFIX: &str = "shrimp-";
const SUFFIX: &str = ".db";
/// Today's copy and the one before.
const KEEP: usize = 2;

/// Copies the database to `dir/shrimp-<day>.db` unless that copy exists, and
/// keeps only the newest copies. Returns the new copy.
pub fn daily(db: &Connection, dir: &Path, day: &str) -> Result<Option<PathBuf>, StorageError> {
    let target = dir.join(format!("{PREFIX}{day}{SUFFIX}"));
    if target.exists() {
        return Ok(None);
    }
    std::fs::create_dir_all(dir).map_err(io)?;
    let temporary = dir.join(format!("{PREFIX}{day}.partial"));
    let _ = std::fs::remove_file(&temporary);
    db.execute("VACUUM INTO ?1", [temporary.to_string_lossy()])?;
    std::fs::rename(&temporary, &target).map_err(io)?;
    for old in copies(dir).into_iter().skip(KEEP) {
        if let Err(error) = std::fs::remove_file(&old) {
            log::warn!("could not delete the old backup {}: {error}", old.display());
        }
    }
    Ok(Some(target))
}

/// The backups, newest first.
pub fn copies(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut copies: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(PREFIX) && name.ends_with(SUFFIX))
        })
        .collect();
    copies.sort();
    copies.reverse();
    copies
}

/// Deletes every backup, e.g. after the history was deleted.
pub fn delete_all(dir: &Path) {
    for copy in copies(dir) {
        if let Err(error) = std::fs::remove_file(&copy) {
            log::warn!("could not delete the backup {}: {error}", copy.display());
        }
    }
}

fn io(error: std::io::Error) -> StorageError {
    StorageError::Invalid(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    #[test]
    fn copies_once_a_day_and_keeps_two() {
        let dir = tempfile::tempdir().unwrap();
        let database = Database::open(&dir.path().join("shrimp.db")).unwrap();
        let backups = dir.path().join("backups");
        database
            .with(|db| {
                assert!(daily(db, &backups, "2026-10-11")?.is_some());
                assert!(daily(db, &backups, "2026-10-11")?.is_none());
                daily(db, &backups, "2026-10-12")?;
                daily(db, &backups, "2026-10-13")?;
                Ok(())
            })
            .unwrap();
        let names: Vec<String> = copies(&backups)
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["shrimp-2026-10-13.db", "shrimp-2026-10-12.db"]);
        let copy = Connection::open(&copies(&backups)[0]).unwrap();
        let tables: i64 = copy
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'history'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 1);
        delete_all(&backups);
        assert!(copies(&backups).is_empty());
    }
}
