//! The SQLite database: one connection behind a mutex, WAL mode, schema
//! migrations kept in `PRAGMA user_version`.

use std::path::Path;
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// A user-facing validation message.
    #[error("{0}")]
    Invalid(String),
    #[error("not found")]
    NotFound,
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("database migration failed: {0}")]
    Migration(#[from] rusqlite_migration::Error),
}

static MIGRATIONS: LazyLock<Migrations<'static>> = LazyLock::new(|| {
    Migrations::new(vec![M::up(
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
    )])
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
    }
}
