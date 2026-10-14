//! Transcript history (HIS-01..10): one row per dictation, newest first.
//! Recordings live next to the database; a row only names its file.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::db::StorageError;

/// How a dictation ended. Mirrors `HistoryStatus` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HistoryStatus {
    /// Pasted into the app.
    Inserted,
    /// Left on the clipboard (no text field, or the paste failed).
    Copied,
    /// Transcribed again from History; nothing was pasted.
    Transcribed,
    /// Transcription failed; the audio can be retried.
    Failed,
    /// No speech was found.
    Empty,
    /// Shrimp quit or crashed while recording; the audio was saved (HIS-06).
    Interrupted,
    /// Cancelled while recording; the audio was saved (HIS-10).
    Cancelled,
}

impl HistoryStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Inserted => "inserted",
            Self::Copied => "copied",
            Self::Transcribed => "transcribed",
            Self::Failed => "failed",
            Self::Empty => "empty",
            Self::Interrupted => "interrupted",
            Self::Cancelled => "cancelled",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "copied" => Self::Copied,
            "transcribed" => Self::Transcribed,
            "failed" => Self::Failed,
            "empty" => Self::Empty,
            "interrupted" => Self::Interrupted,
            "cancelled" => Self::Cancelled,
            _ => Self::Inserted,
        }
    }

    /// Whether the entry has a transcript to copy.
    pub fn has_text(self) -> bool {
        matches!(self, Self::Inserted | Self::Copied | Self::Transcribed)
    }
}

/// One stored dictation. Mirrors `HistoryEntry` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    /// Unix milliseconds.
    pub created_at: i64,
    pub status: HistoryStatus,
    pub app_name: Option<String>,
    /// What the speech engine wrote.
    pub raw_text: String,
    /// What was inserted, after AI edits.
    pub final_text: String,
    /// False when AI edits were off, skipped or failed (FMT-10).
    pub formatted: bool,
    /// Undo AI edit shows the raw text instead (FMT-12).
    pub show_raw: bool,
    /// The language it was transcribed in, a Whisper code.
    pub language: Option<String>,
    pub audio_ms: i64,
    pub word_count: i64,
    /// Why it failed, for failed entries.
    pub error: Option<String>,
    /// The recording's file name in the recordings folder.
    #[serde(skip)]
    pub audio_path: Option<String>,
    #[serde(skip)]
    pub app_id: Option<String>,
}

impl HistoryEntry {
    /// The text the row shows and copies.
    pub fn shown_text(&self) -> &str {
        if self.show_raw {
            &self.raw_text
        } else {
            &self.final_text
        }
    }
}

/// A dictation to store.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewEntry {
    pub created_at: i64,
    pub mode: &'static str,
    pub status: Option<HistoryStatus>,
    pub app_id: Option<String>,
    pub app_name: Option<String>,
    pub raw_text: String,
    pub final_text: String,
    pub formatted: bool,
    pub language: Option<String>,
    pub stt_engine: Option<String>,
    pub llm_provider: Option<String>,
    pub audio_ms: i64,
    pub stt_ms: Option<i64>,
    pub llm_ms: Option<i64>,
    pub word_count: i64,
    pub error: Option<String>,
}

/// A new transcript for an existing entry (Retry, Recover).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retranscribed {
    pub status: HistoryStatus,
    pub raw_text: String,
    pub final_text: String,
    pub formatted: bool,
    pub language: Option<String>,
    pub word_count: i64,
    pub error: Option<String>,
}

const COLUMNS: &str = "id, created_at, status, app_name, raw_text, final_text, formatted, \
                       show_raw, language, audio_ms, word_count, error, audio_path, app_id";

fn entry(row: &Row<'_>) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id: row.get(0)?,
        created_at: row.get(1)?,
        status: HistoryStatus::parse(&row.get::<_, String>(2)?),
        app_name: row.get(3)?,
        raw_text: row.get(4)?,
        final_text: row.get(5)?,
        formatted: row.get(6)?,
        show_raw: row.get(7)?,
        language: row.get(8)?,
        audio_ms: row.get(9)?,
        word_count: row.get(10)?,
        error: row.get(11)?,
        audio_path: row.get(12)?,
        app_id: row.get(13)?,
    })
}

pub fn insert(db: &Connection, new: &NewEntry) -> Result<HistoryEntry, StorageError> {
    let status = new.status.unwrap_or(HistoryStatus::Inserted);
    db.execute(
        "INSERT INTO history (created_at, mode, status, app_id, app_name, raw_text, final_text,
           formatted, language, stt_engine, llm_provider, audio_ms, stt_ms, llm_ms, word_count, error)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            new.created_at,
            new.mode,
            status.as_str(),
            new.app_id,
            new.app_name,
            new.raw_text,
            new.final_text,
            new.formatted,
            new.language,
            new.stt_engine,
            new.llm_provider,
            new.audio_ms,
            new.stt_ms,
            new.llm_ms,
            new.word_count,
            new.error,
        ],
    )?;
    get(db, db.last_insert_rowid())
}

pub fn get(db: &Connection, id: i64) -> Result<HistoryEntry, StorageError> {
    db.query_row(
        &format!("SELECT {COLUMNS} FROM history WHERE id = ?1"),
        [id],
        entry,
    )
    .optional()?
    .ok_or(StorageError::NotFound)
}

/// Newest first; `query` matches the transcript or the app, ignoring ASCII case.
pub fn list(db: &Connection, query: &str, limit: usize) -> Result<Vec<HistoryEntry>, StorageError> {
    let query = query.trim();
    let pattern = format!("%{}%", escape_like(query));
    let mut statement = db.prepare(&format!(
        "SELECT {COLUMNS} FROM history
         WHERE ?1 = '' OR final_text LIKE ?2 ESCAPE '\\' OR raw_text LIKE ?2 ESCAPE '\\'
            OR app_name LIKE ?2 ESCAPE '\\'
         ORDER BY created_at DESC, id DESC LIMIT ?3"
    ))?;
    let entries = statement
        .query_map(params![query, pattern, limit as i64], entry)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(entries)
}

fn escape_like(text: &str) -> String {
    text.chars()
        .flat_map(|c| match c {
            '%' | '_' | '\\' => vec!['\\', c],
            c => vec![c],
        })
        .collect()
}

pub fn set_audio_path(db: &Connection, id: i64, path: Option<&str>) -> Result<(), StorageError> {
    db.execute(
        "UPDATE history SET audio_path = ?2 WHERE id = ?1",
        params![id, path],
    )?;
    Ok(())
}

pub fn set_show_raw(
    db: &Connection,
    id: i64,
    show_raw: bool,
) -> Result<HistoryEntry, StorageError> {
    let changed = db.execute(
        "UPDATE history SET show_raw = ?2 WHERE id = ?1",
        params![id, show_raw],
    )?;
    if changed == 0 {
        return Err(StorageError::NotFound);
    }
    get(db, id)
}

pub fn update_transcript(
    db: &Connection,
    id: i64,
    result: &Retranscribed,
) -> Result<HistoryEntry, StorageError> {
    let changed = db.execute(
        "UPDATE history SET status = ?2, raw_text = ?3, final_text = ?4, formatted = ?5,
           language = ?6, word_count = ?7, error = ?8, show_raw = 0
         WHERE id = ?1",
        params![
            id,
            result.status.as_str(),
            result.raw_text,
            result.final_text,
            result.formatted,
            result.language,
            result.word_count,
            result.error,
        ],
    )?;
    if changed == 0 {
        return Err(StorageError::NotFound);
    }
    get(db, id)
}

/// Deletes one entry and returns its recording's file name.
pub fn delete(db: &Connection, id: i64) -> Result<Option<String>, StorageError> {
    let path: Option<Option<String>> = db
        .query_row(
            "DELETE FROM history WHERE id = ?1 RETURNING audio_path",
            [id],
            |row| row.get(0),
        )
        .optional()?;
    path.ok_or(StorageError::NotFound)
}

/// Deletes entries created before `cutoff` (unix ms), or every entry with
/// `None`, and returns their recordings' file names.
pub fn delete_before(db: &Connection, cutoff: Option<i64>) -> Result<Vec<String>, StorageError> {
    let cutoff = cutoff.unwrap_or(i64::MAX);
    let mut statement =
        db.prepare("DELETE FROM history WHERE created_at < ?1 RETURNING audio_path")?;
    let paths = statement
        .query_map([cutoff], |row| row.get::<_, Option<String>>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(paths.into_iter().flatten().collect())
}

/// Forgets the recordings of entries created before `cutoff` and returns
/// their file names, for deletion (HIS-04).
pub fn expire_audio(db: &mut Connection, cutoff: i64) -> Result<Vec<String>, StorageError> {
    let transaction = db.transaction()?;
    let paths = {
        let mut read = transaction.prepare(
            "SELECT audio_path FROM history WHERE created_at < ?1 AND audio_path IS NOT NULL",
        )?;
        read.query_map([cutoff], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    transaction.execute(
        "UPDATE history SET audio_path = NULL WHERE created_at < ?1 AND audio_path IS NOT NULL",
        [cutoff],
    )?;
    transaction.commit()?;
    Ok(paths)
}

/// Every recording file name still referenced.
pub fn audio_paths(db: &Connection) -> Result<Vec<String>, StorageError> {
    let mut statement =
        db.prepare("SELECT audio_path FROM history WHERE audio_path IS NOT NULL")?;
    let paths = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    fn new(text: &str, at: i64) -> NewEntry {
        NewEntry {
            created_at: at,
            mode: "dictation",
            raw_text: format!("um {text}"),
            final_text: text.into(),
            formatted: true,
            app_name: Some("TextEdit".into()),
            word_count: text.split_whitespace().count() as i64,
            ..NewEntry::default()
        }
    }

    #[test]
    fn lists_newest_first_and_searches() {
        let database = Database::open_in_memory().unwrap();
        database
            .with(|db| {
                insert(db, &new("coffee at three", 1_000))?;
                insert(db, &new("ship 100% of it", 2_000))?;
                let all = list(db, "", 10)?;
                assert_eq!(all[0].final_text, "ship 100% of it");
                assert_eq!(list(db, "COFFEE", 10)?.len(), 1);
                assert_eq!(list(db, "100%", 10)?.len(), 1);
                assert_eq!(list(db, "%", 10)?.len(), 1, "% is literal");
                assert_eq!(list(db, "textedit", 10)?.len(), 2);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn undo_ai_edit_shows_the_raw_text() {
        let database = Database::open_in_memory().unwrap();
        database
            .with(|db| {
                let id = insert(db, &new("hello", 1))?.id;
                assert_eq!(set_show_raw(db, id, true)?.shown_text(), "um hello");
                assert_eq!(set_show_raw(db, id, false)?.shown_text(), "hello");
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn expiring_and_deleting_return_the_recordings() {
        let database = Database::open_in_memory().unwrap();
        database
            .with(|db| {
                let old = insert(db, &new("old", 1_000))?.id;
                let recent = insert(db, &new("recent", 9_000))?.id;
                set_audio_path(db, old, Some("1.wav"))?;
                set_audio_path(db, recent, Some("2.wav"))?;
                assert_eq!(expire_audio(db, 5_000)?, ["1.wav"]);
                assert_eq!(get(db, old)?.audio_path, None);
                assert_eq!(audio_paths(db)?, ["2.wav"]);
                assert_eq!(delete_before(db, Some(10_000))?, ["2.wav"]);
                assert!(list(db, "", 10)?.is_empty());
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn statuses_are_stored_as_text() {
        let database = Database::open_in_memory().unwrap();
        database
            .with(|db| {
                let mut cancelled = new("", 1);
                cancelled.status = Some(HistoryStatus::Cancelled);
                let id = insert(db, &cancelled)?.id;
                let stored: String =
                    db.query_row("SELECT status FROM history WHERE id = ?1", [id], |row| {
                        row.get(0)
                    })?;
                assert_eq!(stored, "cancelled");
                assert_eq!(get(db, id)?.status, HistoryStatus::Cancelled);
                assert!(!HistoryStatus::Cancelled.has_text());
                Ok(())
            })
            .unwrap();
        assert_eq!(
            serde_json::to_value(HistoryStatus::Cancelled).unwrap(),
            "cancelled"
        );
    }

    #[test]
    fn a_retry_replaces_the_transcript() {
        let database = Database::open_in_memory().unwrap();
        database
            .with(|db| {
                let mut failed = new("", 1);
                failed.status = Some(HistoryStatus::Failed);
                let id = insert(db, &failed)?.id;
                let entry = update_transcript(
                    db,
                    id,
                    &Retranscribed {
                        status: HistoryStatus::Transcribed,
                        raw_text: "hi".into(),
                        final_text: "Hi.".into(),
                        formatted: true,
                        language: Some("en".into()),
                        word_count: 1,
                        error: None,
                    },
                )?;
                assert_eq!(entry.status, HistoryStatus::Transcribed);
                assert_eq!(entry.final_text, "Hi.");
                assert!(matches!(delete(db, id), Ok(None)));
                assert!(matches!(delete(db, id), Err(StorageError::NotFound)));
                Ok(())
            })
            .unwrap();
    }
}
