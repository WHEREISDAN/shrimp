//! Usage stats (STA-01..05). They live apart from history, so deleting or
//! not storing transcripts never resets them (STA-05).

use rusqlite::{Connection, OptionalExtension, params};

use super::db::StorageError;

/// Recordings longer than this don't count towards speed.
pub const MAX_SPEED_RECORDING_MS: i64 = 400_000;
/// Speed is measured over this many recent dictations.
pub const SPEED_WINDOW: i64 = 100;

/// One dictation with words, counted where it happened (local time).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counted {
    pub created_at: i64,
    /// Local date, YYYY-MM-DD.
    pub day: String,
    /// 0 is Sunday.
    pub weekday: u32,
    pub hour: u32,
    pub app_id: Option<String>,
    pub app_name: Option<String>,
    pub words: i64,
    pub audio_ms: i64,
    /// Speech without leading and trailing silence.
    pub speech_ms: i64,
    /// Words the AI edit removed or changed.
    pub fixes: i64,
    /// Dictionary corrections and snippets applied.
    pub dictionary_fixes: i64,
}

pub fn record(db: &mut Connection, counted: &Counted) -> Result<(), StorageError> {
    if counted.words <= 0 {
        return Ok(());
    }
    let transaction = db.transaction()?;
    transaction.execute(
        "INSERT INTO daily_stats (day, dictations, words, audio_ms, fixes, dictionary_fixes)
         VALUES (?1, 1, ?2, ?3, ?4, ?5)
         ON CONFLICT(day) DO UPDATE SET dictations = dictations + 1, words = words + ?2,
           audio_ms = audio_ms + ?3, fixes = fixes + ?4, dictionary_fixes = dictionary_fixes + ?5",
        params![
            counted.day,
            counted.words,
            counted.audio_ms,
            counted.fixes,
            counted.dictionary_fixes
        ],
    )?;
    if let Some(app_id) = &counted.app_id {
        transaction.execute(
            "INSERT INTO app_usage (day, app_id, app_name, words, dictations)
             VALUES (?1, ?2, ?3, ?4, 1)
             ON CONFLICT(day, app_id) DO UPDATE SET words = words + ?4,
               dictations = dictations + 1, app_name = ?3",
            params![
                counted.day,
                app_id,
                counted.app_name.as_deref().unwrap_or(app_id),
                counted.words
            ],
        )?;
    }
    transaction.execute(
        "INSERT INTO hourly_activity (weekday, hour, words) VALUES (?1, ?2, ?3)
         ON CONFLICT(weekday, hour) DO UPDATE SET words = words + ?3",
        params![counted.weekday, counted.hour, counted.words],
    )?;
    if counted.audio_ms < MAX_SPEED_RECORDING_MS && counted.speech_ms > 0 {
        transaction.execute(
            "INSERT INTO recent_speed (created_at, words, speech_ms) VALUES (?1, ?2, ?3)",
            params![counted.created_at, counted.words, counted.speech_ms],
        )?;
        transaction.execute(
            "DELETE FROM recent_speed WHERE id NOT IN
               (SELECT id FROM recent_speed ORDER BY created_at DESC, id DESC LIMIT ?1)",
            [SPEED_WINDOW],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

/// Every word dictated so far.
pub fn total_words(db: &Connection) -> Result<i64, StorageError> {
    Ok(
        db.query_row("SELECT IFNULL(SUM(words), 0) FROM daily_stats", [], |row| {
            row.get(0)
        })?,
    )
}

pub fn meta(db: &Connection, key: &str) -> Result<Option<String>, StorageError> {
    Ok(db
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()?)
}

pub fn set_meta(db: &Connection, key: &str, value: &str) -> Result<(), StorageError> {
    db.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = ?2",
        params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn counted(day: &str, words: i64) -> Counted {
    Counted {
        created_at: 0,
        day: day.into(),
        weekday: 1,
        hour: 9,
        app_id: Some("com.apple.TextEdit".into()),
        app_name: Some("TextEdit".into()),
        words,
        audio_ms: 10_000,
        speech_ms: 8_000,
        fixes: 2,
        dictionary_fixes: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    #[test]
    fn counts_add_up_per_day_and_speed_keeps_the_last_hundred() {
        let database = Database::open_in_memory().unwrap();
        database
            .with(|db| {
                for _ in 0..105 {
                    record(db, &counted("2026-10-12", 10))?;
                }
                record(db, &counted("2026-10-13", 0))?;
                assert_eq!(total_words(db)?, 1_050);
                let speeds: i64 =
                    db.query_row("SELECT COUNT(*) FROM recent_speed", [], |row| row.get(0))?;
                assert_eq!(speeds, SPEED_WINDOW);
                let days: i64 =
                    db.query_row("SELECT COUNT(*) FROM daily_stats", [], |row| row.get(0))?;
                assert_eq!(days, 1, "a dictation without words isn't counted");
                set_meta(db, "k", "v")?;
                assert_eq!(meta(db, "k")?.as_deref(), Some("v"));
                Ok(())
            })
            .unwrap();
    }
}
