//! Usage stats (STA-01..06). They live apart from history, so deleting or
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
    /// The usage category's key ("emails"), when known (STA-06).
    pub usage: Option<&'static str>,
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
    if let Some(category) = counted.usage {
        transaction.execute(
            "INSERT INTO usage_categories (category, words, dictations) VALUES (?1, ?2, 1)
             ON CONFLICT(category) DO UPDATE SET words = words + ?2,
               dictations = dictations + 1",
            params![category, counted.words],
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

/// Sums over every day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Totals {
    pub words: i64,
    pub dictations: i64,
    pub fixes: i64,
    pub dictionary_fixes: i64,
}

pub fn totals(db: &Connection) -> Result<Totals, StorageError> {
    Ok(db.query_row(
        "SELECT IFNULL(SUM(words), 0), IFNULL(SUM(dictations), 0), IFNULL(SUM(fixes), 0),
                IFNULL(SUM(dictionary_fixes), 0)
         FROM daily_stats",
        [],
        |row| {
            Ok(Totals {
                words: row.get(0)?,
                dictations: row.get(1)?,
                fixes: row.get(2)?,
                dictionary_fixes: row.get(3)?,
            })
        },
    )?)
}

/// Words per minute over the recent dictations, rounded; 0 without any.
pub fn speed(db: &Connection) -> Result<i64, StorageError> {
    let (words, speech_ms): (i64, i64) = db.query_row(
        "SELECT IFNULL(SUM(words), 0), IFNULL(SUM(speech_ms), 0) FROM recent_speed",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if speech_ms <= 0 {
        return Ok(0);
    }
    Ok(((words as f64) * 60_000.0 / speech_ms as f64).round() as i64)
}

/// Words dictated from `from` to `to` (local dates, inclusive).
pub fn words_between(db: &Connection, from: &str, to: &str) -> Result<i64, StorageError> {
    Ok(db.query_row(
        "SELECT IFNULL(SUM(words), 0) FROM daily_stats WHERE day BETWEEN ?1 AND ?2",
        params![from, to],
        |row| row.get(0),
    )?)
}

/// One active day, for the heatmap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Day {
    pub day: String,
    pub words: i64,
    pub dictations: i64,
    pub apps: i64,
    pub top_app: Option<String>,
}

/// Every day with words since `from` (inclusive), oldest first.
pub fn days_since(db: &Connection, from: &str) -> Result<Vec<Day>, StorageError> {
    let mut statement = db.prepare(
        "SELECT d.day, d.words, d.dictations,
                (SELECT COUNT(*) FROM app_usage a WHERE a.day = d.day),
                (SELECT a.app_name FROM app_usage a WHERE a.day = d.day
                 ORDER BY a.words DESC, a.app_name LIMIT 1)
         FROM daily_stats d WHERE d.day >= ?1 AND d.words > 0 ORDER BY d.day",
    )?;
    let days = statement
        .query_map([from], |row| {
            Ok(Day {
                day: row.get(0)?,
                words: row.get(1)?,
                dictations: row.get(2)?,
                apps: row.get(3)?,
                top_app: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(days)
}

/// Every day with words, oldest first, for streaks.
pub fn active_days(db: &Connection) -> Result<Vec<String>, StorageError> {
    let mut statement = db.prepare("SELECT day FROM daily_stats WHERE words > 0 ORDER BY day")?;
    let days = statement
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(days)
}

/// One app over all time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppTotal {
    pub app_id: String,
    pub name: String,
    pub words: i64,
    pub dictations: i64,
}

/// Apps by words dictated, most first.
pub fn apps(db: &Connection) -> Result<Vec<AppTotal>, StorageError> {
    let mut statement = db.prepare(
        "SELECT app_id, MAX(app_name), SUM(words), SUM(dictations) FROM app_usage
         GROUP BY app_id ORDER BY SUM(words) DESC, MAX(app_name)",
    )?;
    let apps = statement
        .query_map([], |row| {
            Ok(AppTotal {
                app_id: row.get(0)?,
                name: row.get(1)?,
                words: row.get(2)?,
                dictations: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(apps)
}

/// One usage category over all time (STA-06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryTotal {
    pub category: String,
    pub words: i64,
    pub dictations: i64,
}

/// Every usage category with words, in no particular order.
pub fn usage(db: &Connection) -> Result<Vec<CategoryTotal>, StorageError> {
    let mut statement = db.prepare("SELECT category, words, dictations FROM usage_categories")?;
    let categories = statement
        .query_map([], |row| {
            Ok(CategoryTotal {
                category: row.get(0)?,
                words: row.get(1)?,
                dictations: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(categories)
}

/// Words by weekday (0 is Sunday) and hour.
pub fn hourly(db: &Connection) -> Result<[[i64; 24]; 7], StorageError> {
    let mut grid = [[0; 24]; 7];
    let mut statement = db.prepare("SELECT weekday, hour, words FROM hourly_activity")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    for row in rows {
        let (weekday, hour, words) = row?;
        let (Ok(weekday), Ok(hour)) = (usize::try_from(weekday), usize::try_from(hour)) else {
            continue;
        };
        if let Some(cell) = grid.get_mut(weekday).and_then(|day| day.get_mut(hour)) {
            *cell = words;
        }
    }
    Ok(grid)
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
        usage: Some("documents"),
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
                let totals = totals(db)?;
                assert_eq!(
                    (totals.words, totals.dictations, totals.fixes),
                    (1_050, 105, 210)
                );
                assert_eq!(speed(db)?, 75, "10 words in 8 s");
                assert_eq!(words_between(db, "2026-10-12", "2026-10-13")?, 1_050);
                let days = days_since(db, "2026-01-01")?;
                assert_eq!(days.len(), 1);
                assert_eq!(days[0].top_app.as_deref(), Some("TextEdit"));
                assert_eq!(days[0].apps, 1);
                assert_eq!(apps(db)?[0].words, 1_050);
                assert_eq!(hourly(db)?[1][9], 1_050);
                assert_eq!(
                    usage(db)?,
                    [CategoryTotal {
                        category: "documents".into(),
                        words: 1_050,
                        dictations: 105
                    }]
                );
                assert_eq!(active_days(db)?, ["2026-10-12"]);
                set_meta(db, "k", "v")?;
                assert_eq!(meta(db, "k")?.as_deref(), Some("v"));
                Ok(())
            })
            .unwrap();
    }
}
