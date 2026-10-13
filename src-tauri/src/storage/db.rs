//! The SQLite database: one connection behind a mutex, WAL mode, schema
//! migrations kept in `PRAGMA user_version`.

use std::path::Path;
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::t;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// A user-facing validation message.
    #[error("{0}")]
    Invalid(String),
    #[error("{}", t!("error.storage.notFound"))]
    NotFound,
    #[error("{}", t!("error.storage.database", details = .0))]
    Sqlite(#[from] rusqlite::Error),
    #[error("{}", t!("error.storage.migration", details = .0))]
    Migration(#[from] rusqlite_migration::Error),
}

static MIGRATIONS: LazyLock<Migrations<'static>> = LazyLock::new(|| {
    Migrations::new(vec![
        M::up(
            "CREATE TABLE dictionary (
           id          INTEGER PRIMARY KEY,
           term        TEXT    NOT NULL,
           misspelling TEXT,
           source      TEXT    NOT NULL,
           pinned      INTEGER NOT NULL DEFAULT 0,
           uses        INTEGER NOT NULL DEFAULT 0,
           created_at  INTEGER NOT NULL
         );
         CREATE UNIQUE INDEX dictionary_key
           ON dictionary(misspelling IS NOT NULL, IFNULL(misspelling, term) COLLATE NOCASE);
         CREATE TABLE dictionary_rejected (term TEXT PRIMARY KEY COLLATE NOCASE);
         CREATE TABLE snippets (
           id         INTEGER PRIMARY KEY,
           trigger    TEXT    NOT NULL UNIQUE COLLATE NOCASE,
           expansion  TEXT    NOT NULL,
           created_at INTEGER NOT NULL,
           updated_at INTEGER NOT NULL
         );",
        ),
        // History and the stats that outlive it (HIS-01..08, STA-01..05).
        M::up(
            "CREATE TABLE history (
           id           INTEGER PRIMARY KEY,
           created_at   INTEGER NOT NULL,
           mode         TEXT    NOT NULL,
           status       TEXT    NOT NULL,
           app_id       TEXT,
           app_name     TEXT,
           raw_text     TEXT    NOT NULL DEFAULT '',
           final_text   TEXT    NOT NULL DEFAULT '',
           formatted    INTEGER NOT NULL DEFAULT 1,
           show_raw     INTEGER NOT NULL DEFAULT 0,
           language     TEXT,
           stt_engine   TEXT,
           llm_provider TEXT,
           audio_ms     INTEGER NOT NULL DEFAULT 0,
           stt_ms       INTEGER,
           llm_ms       INTEGER,
           word_count   INTEGER NOT NULL DEFAULT 0,
           audio_path   TEXT,
           error        TEXT
         );
         CREATE INDEX history_created_at ON history(created_at DESC);
         CREATE TABLE daily_stats (
           day              TEXT PRIMARY KEY,
           dictations       INTEGER NOT NULL DEFAULT 0,
           words            INTEGER NOT NULL DEFAULT 0,
           audio_ms         INTEGER NOT NULL DEFAULT 0,
           fixes            INTEGER NOT NULL DEFAULT 0,
           dictionary_fixes INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE app_usage (
           day        TEXT    NOT NULL,
           app_id     TEXT    NOT NULL,
           app_name   TEXT    NOT NULL,
           words      INTEGER NOT NULL DEFAULT 0,
           dictations INTEGER NOT NULL DEFAULT 0,
           PRIMARY KEY (day, app_id)
         );
         CREATE TABLE hourly_activity (
           weekday INTEGER NOT NULL,
           hour    INTEGER NOT NULL,
           words   INTEGER NOT NULL DEFAULT 0,
           PRIMARY KEY (weekday, hour)
         );
         CREATE TABLE recent_speed (
           id         INTEGER PRIMARY KEY,
           created_at INTEGER NOT NULL,
           words      INTEGER NOT NULL,
           speech_ms  INTEGER NOT NULL
         );
         CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        ),
    ])
});

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|err| StorageError::Invalid(err.to_string()))?;
        }
        let mut connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        MIGRATIONS.to_latest(&mut connection)?;
        Ok(Self::from(connection))
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let mut connection = Connection::open_in_memory()?;
        MIGRATIONS.to_latest(&mut connection)?;
        Ok(Self::from(connection))
    }

    /// SQLite's quick integrity check (APP-09).
    pub fn check(&self) -> Result<(), StorageError> {
        let result: String =
            self.with(|db| Ok(db.query_row("PRAGMA quick_check", [], |row| row.get(0))?))?;
        if result == "ok" {
            Ok(())
        } else {
            Err(StorageError::Invalid(result))
        }
    }

    /// Runs `work` with the connection. Call from the blocking pool.
    pub fn with<T>(
        &self,
        work: impl FnOnce(&mut Connection) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let mut connection = self
            .connection
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        work(&mut connection)
    }
}

impl From<Connection> for Database {
    fn from(connection: Connection) -> Self {
        Self {
            connection: Mutex::new(connection),
        }
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        MIGRATIONS.validate().unwrap();
    }

    #[test]
    fn opens_a_file_in_wal_mode() {
        let dir = tempfile::tempdir().unwrap();
        let database = Database::open(&dir.path().join("shrimp.db")).unwrap();
        let mode: String = database
            .with(|db| Ok(db.query_row("PRAGMA journal_mode", [], |row| row.get(0))?))
            .unwrap();
        assert_eq!(mode, "wal");
        database.check().unwrap();
    }

    #[test]
    fn a_file_that_isnt_a_database_fails_to_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shrimp.db");
        std::fs::write(&path, vec![0x42; 8192]).unwrap();
        assert!(Database::open(&path).is_err());
    }
}
