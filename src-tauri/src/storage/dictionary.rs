//! The personal dictionary: vocabulary words and misspelling corrections
//! (DIC-01..06).

use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::db::{StorageError, now_ms};
use super::import::{
    CsvRow, MAX_IMPORT_ENTRIES, SkippedRow, TOO_MANY, check_size, parse_dictionary_csv,
};

pub const MAX_TERM_CHARS: usize = 60;

/// Mirrors `DictionaryEntry` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntry {
    pub id: i64,
    /// The correct spelling.
    pub term: String,
    /// Set for corrections: what the speech engine writes instead.
    pub misspelling: Option<String>,
    pub source: EntrySource,
    pub pinned: bool,
    pub uses: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntrySource {
    Manual,
    Learned,
    Imported,
}

impl EntrySource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Learned => "learned",
            Self::Imported => "imported",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "learned" => Self::Learned,
            "imported" => Self::Imported,
            _ => Self::Manual,
        }
    }
}

/// A word or correction as entered. Mirrors `DictionaryInput` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryInput {
    pub term: String,
    #[serde(default)]
    pub misspelling: Option<String>,
}

impl DictionaryInput {
    /// Trims and validates; the error is the message the UI shows.
    pub fn normalized(&self) -> Result<Self, String> {
        let term = check_word(&self.term)?;
        let misspelling = self.misspelling.as_deref().map(check_word).transpose()?;
        if misspelling.as_deref() == Some(term.as_str()) {
            return Err("The correction is the same as the misspelling.".into());
        }
        Ok(Self { term, misspelling })
    }

    /// The text a duplicate is found by: the misspelling for corrections.
    fn key(&self) -> (bool, String) {
        match &self.misspelling {
            Some(misspelling) => (true, misspelling.to_lowercase()),
            None => (false, self.term.to_lowercase()),
        }
    }

    fn display(&self) -> &str {
        self.misspelling.as_deref().unwrap_or(&self.term)
    }
}

fn check_word(text: &str) -> Result<String, String> {
    let word = text.trim();
    if word.is_empty() {
        return Err("Word cannot be empty".into());
    }
    if word.chars().count() > MAX_TERM_CHARS {
        return Err(format!("Use at most {MAX_TERM_CHARS} characters."));
    }
    if !word.chars().any(char::is_alphanumeric) {
        return Err("Use at least one letter or number.".into());
    }
    Ok(word.to_owned())
}

fn duplicate(input: &DictionaryInput) -> StorageError {
    StorageError::Invalid(format!(
        "\"{}\" already exists in your personal dictionary",
        input.display()
    ))
}

const COLUMNS: &str = "id, term, misspelling, source, pinned, uses, created_at";

fn entry(row: &Row<'_>) -> rusqlite::Result<DictionaryEntry> {
    Ok(DictionaryEntry {
        id: row.get(0)?,
        term: row.get(1)?,
        misspelling: row.get(2)?,
        source: EntrySource::parse(&row.get::<_, String>(3)?),
        pinned: row.get(4)?,
        uses: row.get(5)?,
        created_at: row.get(6)?,
    })
}

/// Pinned first, then the most used, then the newest (DIC-04).
pub fn list(db: &Connection) -> Result<Vec<DictionaryEntry>, StorageError> {
    let mut statement = db.prepare(&format!(
        "SELECT {COLUMNS} FROM dictionary ORDER BY pinned DESC, uses DESC, created_at DESC, id DESC"
    ))?;
    let entries = statement
        .query_map([], entry)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(entries)
}

pub fn get(db: &Connection, id: i64) -> Result<DictionaryEntry, StorageError> {
    db.query_row(
        &format!("SELECT {COLUMNS} FROM dictionary WHERE id = ?1"),
        [id],
        entry,
    )
    .optional()?
    .ok_or(StorageError::NotFound)
}

fn find_key(db: &Connection, input: &DictionaryInput) -> Result<Option<i64>, StorageError> {
    let (is_correction, key) = input.key();
    Ok(db
        .query_row(
            "SELECT id FROM dictionary
             WHERE (misspelling IS NOT NULL) = ?1 AND IFNULL(misspelling, term) = ?2 COLLATE NOCASE",
            params![is_correction, key],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn add(
    db: &Connection,
    input: &DictionaryInput,
    source: EntrySource,
) -> Result<DictionaryEntry, StorageError> {
    let input = input.normalized().map_err(StorageError::Invalid)?;
    if find_key(db, &input)?.is_some() {
        return Err(duplicate(&input));
    }
    db.execute(
        "INSERT INTO dictionary (term, misspelling, source, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![input.term, input.misspelling, source.as_str(), now_ms()],
    )?;
    let id = db.last_insert_rowid();
    if source == EntrySource::Manual {
        // Added by hand: auto-learn may know it again.
        db.execute(
            "DELETE FROM dictionary_rejected WHERE term = ?1 COLLATE NOCASE",
            [&input.term],
        )?;
    }
    get(db, id)
}

/// Adds words learned from the user's corrections (DIC-07). Words already
/// in the dictionary, and words the user deleted, are skipped.
pub fn learn(db: &mut Connection, words: &[String]) -> Result<Vec<DictionaryEntry>, StorageError> {
    let transaction = db.transaction()?;
    let mut learned = Vec::new();
    for word in words {
        let input = DictionaryInput {
            term: word.clone(),
            misspelling: None,
        };
        let Ok(input) = input.normalized() else {
            continue;
        };
        let rejected: bool = transaction.query_row(
            "SELECT EXISTS (SELECT 1 FROM dictionary_rejected WHERE term = ?1 COLLATE NOCASE)",
            [&input.term],
            |row| row.get(0),
        )?;
        let known: bool = transaction.query_row(
            "SELECT EXISTS (SELECT 1 FROM dictionary WHERE term = ?1 COLLATE NOCASE)",
            [&input.term],
            |row| row.get(0),
        )?;
        if rejected || known || find_key(&transaction, &input)?.is_some() {
            continue;
        }
        learned.push(add(&transaction, &input, EntrySource::Learned)?);
    }
    transaction.commit()?;
    Ok(learned)
}

/// Edits a word or correction; `pinned` is left alone when `None`.
pub fn update(
    db: &Connection,
    id: i64,
    input: &DictionaryInput,
    pinned: Option<bool>,
) -> Result<DictionaryEntry, StorageError> {
    let current = get(db, id)?;
    let input = input.normalized().map_err(StorageError::Invalid)?;
    if find_key(db, &input)?.is_some_and(|other| other != id) {
        return Err(duplicate(&input));
    }
    db.execute(
        "UPDATE dictionary SET term = ?1, misspelling = ?2, pinned = ?3 WHERE id = ?4",
        params![
            input.term,
            input.misspelling,
            pinned.unwrap_or(current.pinned),
            id
        ],
    )?;
    get(db, id)
}

pub fn set_pinned(db: &Connection, id: i64, pinned: bool) -> Result<DictionaryEntry, StorageError> {
    if db.execute(
        "UPDATE dictionary SET pinned = ?1 WHERE id = ?2",
        params![pinned, id],
    )? == 0
    {
        return Err(StorageError::NotFound);
    }
    get(db, id)
}

/// Deletes entries; auto-learn never adds their words again (DIC-07).
pub fn delete(db: &mut Connection, ids: &[i64]) -> Result<usize, StorageError> {
    let transaction = db.transaction()?;
    let mut deleted = 0;
    for id in ids {
        if let Ok(entry) = get(&transaction, *id) {
            transaction.execute(
                "INSERT OR IGNORE INTO dictionary_rejected (term) VALUES (?1)",
                [&entry.term],
            )?;
        }
        deleted += transaction.execute("DELETE FROM dictionary WHERE id = ?1", [id])?;
    }
    transaction.commit()?;
    Ok(deleted)
}

/// Counts one use for each entry a dictation contained.
pub fn record_uses(db: &mut Connection, ids: &[i64]) -> Result<(), StorageError> {
    let transaction = db.transaction()?;
    for id in ids {
        transaction.execute("UPDATE dictionary SET uses = uses + 1 WHERE id = ?1", [id])?;
    }
    transaction.commit()?;
    Ok(())
}

/// What an import would do. Mirrors `DictionaryImportPreview` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub added: Vec<DictionaryInput>,
    pub duplicates: Vec<DictionaryInput>,
    pub skipped: Vec<SkippedRow>,
}

pub fn preview_import(db: &Connection, csv: &str) -> Result<ImportPreview, StorageError> {
    check_size(csv).map_err(StorageError::Invalid)?;
    let (rows, mut skipped) = parse_dictionary_csv(csv);
    if rows.is_empty() && skipped.is_empty() {
        return Err(StorageError::Invalid(
            "No dictionary items found in file".into(),
        ));
    }
    let mut seen = HashSet::new();
    let mut added = Vec::new();
    let mut duplicates = Vec::new();
    for (line, row) in rows {
        let input = match row {
            CsvRow::Word(term) => DictionaryInput {
                term,
                misspelling: None,
            },
            CsvRow::Correction { misspelling, term } => DictionaryInput {
                term,
                misspelling: Some(misspelling),
            },
        };
        let input = match input.normalized() {
            Ok(input) => input,
            Err(reason) => {
                skipped.push(SkippedRow { line, reason });
                continue;
            }
        };
        if !seen.insert(input.key()) || find_key(db, &input)?.is_some() {
            duplicates.push(input);
        } else {
            added.push(input);
        }
    }
    if added.len() > MAX_IMPORT_ENTRIES {
        return Err(StorageError::Invalid(TOO_MANY.into()));
    }
    skipped.sort_by_key(|row| row.line);
    Ok(ImportPreview {
        added,
        duplicates,
        skipped,
    })
}

/// Adds every new row, marked "Imported from file". Returns how many.
pub fn import(db: &mut Connection, csv: &str) -> Result<usize, StorageError> {
    let preview = preview_import(db, csv)?;
    let transaction = db.transaction()?;
    for input in &preview.added {
        add(&transaction, input, EntrySource::Imported)?;
    }
    transaction.commit()?;
    Ok(preview.added.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    fn word(term: &str) -> DictionaryInput {
        DictionaryInput {
            term: term.into(),
            misspelling: None,
        }
    }

    fn correction(misspelling: &str, term: &str) -> DictionaryInput {
        DictionaryInput {
            term: term.into(),
            misspelling: Some(misspelling.into()),
        }
    }

    fn with_db(test: impl FnOnce(&mut Connection)) {
        Database::open_in_memory()
            .unwrap()
            .with(|db| {
                test(db);
                Ok(())
            })
            .unwrap();
    }

    fn message(error: StorageError) -> String {
        error.to_string()
    }

    #[test]
    fn adds_words_and_rejects_bad_ones() {
        with_db(|db| {
            let shrimp = add(db, &word("  Shrimp "), EntrySource::Manual).unwrap();
            assert_eq!(shrimp.term, "Shrimp");
            assert_eq!(shrimp.source, EntrySource::Manual);
            assert_eq!(
                message(add(db, &word("shrimp"), EntrySource::Manual).unwrap_err()),
                "\"shrimp\" already exists in your personal dictionary"
            );
            assert_eq!(
                message(add(db, &word("   "), EntrySource::Manual).unwrap_err()),
                "Word cannot be empty"
            );
            assert!(add(db, &word("?!…"), EntrySource::Manual).is_err());
            assert!(add(db, &word(&"a".repeat(61)), EntrySource::Manual).is_err());
            assert!(add(db, &word(&"a".repeat(60)), EntrySource::Manual).is_ok());
        });
    }

    #[test]
    fn a_misspelling_has_one_correction() {
        with_db(|db| {
            add(db, &correction("jira", "Jira"), EntrySource::Manual).unwrap();
            assert!(add(db, &correction("JIRA", "Jirah"), EntrySource::Manual).is_err());
            add(db, &word("jira"), EntrySource::Manual).unwrap();
            assert!(add(db, &correction("same", "same"), EntrySource::Manual).is_err());
        });
    }

    #[test]
    fn lists_pinned_then_most_used() {
        with_db(|db| {
            let a = add(db, &word("Alpha"), EntrySource::Manual).unwrap();
            let b = add(db, &word("Beta"), EntrySource::Manual).unwrap();
            let c = add(db, &word("Gamma"), EntrySource::Manual).unwrap();
            record_uses(db, &[a.id, a.id, b.id]).unwrap();
            set_pinned(db, c.id, true).unwrap();
            let order: Vec<String> = list(db).unwrap().into_iter().map(|e| e.term).collect();
            assert_eq!(order, ["Gamma", "Alpha", "Beta"]);
        });
    }

    #[test]
    fn updates_and_deletes() {
        with_db(|db| {
            let a = add(db, &word("Alpha"), EntrySource::Manual).unwrap();
            let b = add(db, &word("Beta"), EntrySource::Manual).unwrap();
            assert!(update(db, b.id, &word("alpha"), None).is_err());
            let edited = update(db, b.id, &correction("beda", "Beta"), Some(true)).unwrap();
            assert_eq!(edited.misspelling.as_deref(), Some("beda"));
            assert!(edited.pinned);
            assert_eq!(delete(db, &[a.id, b.id, 99]).unwrap(), 2);
            assert!(list(db).unwrap().is_empty());
        });
    }

    #[test]
    fn import_previews_duplicates_and_marks_rows() {
        with_db(|db| {
            add(db, &word("Shrimp"), EntrySource::Manual).unwrap();
            let csv = "shrimp\nTauri\ntauri\nwhispr,Wispr\n!!!\n";
            let preview = preview_import(db, csv).unwrap();
            assert_eq!(
                preview.added,
                [word("Tauri"), correction("whispr", "Wispr")]
            );
            assert_eq!(preview.duplicates, [word("shrimp"), word("tauri")]);
            assert_eq!(preview.skipped.len(), 1);
            assert_eq!(import(db, csv).unwrap(), 2);
            let imported: Vec<EntrySource> = list(db)
                .unwrap()
                .into_iter()
                .filter(|entry| entry.term != "Shrimp")
                .map(|entry| entry.source)
                .collect();
            assert_eq!(imported, [EntrySource::Imported, EntrySource::Imported]);
        });
    }

    #[test]
    fn import_limits_are_enforced() {
        with_db(|db| {
            let csv: String = (0..1001).map(|i| format!("word{i}\n")).collect();
            assert_eq!(
                message(preview_import(db, &csv).unwrap_err()),
                "Cannot import more than 1,000 entries. Please try again."
            );
            let csv: String = (0..1000).map(|i| format!("word{i}\n")).collect();
            assert_eq!(preview_import(db, &csv).unwrap().added.len(), 1000);
            assert_eq!(
                message(preview_import(db, "\n,\n").unwrap_err()),
                "No dictionary items found in file"
            );
        });
    }

    #[test]
    fn learned_words_skip_known_and_deleted_ones() {
        let database = Database::open_in_memory().unwrap();
        database
            .with(|db| {
                add(db, &word("Kubernetes"), EntrySource::Manual)?;
                let learned = learn(db, &["Nguyen".into(), "kubernetes".into(), "!!".into()])?;
                assert_eq!(learned.len(), 1);
                assert_eq!(learned[0].term, "Nguyen");
                assert_eq!(learned[0].source, EntrySource::Learned);
                delete(db, &[learned[0].id])?;
                assert!(
                    learn(db, &["Nguyen".into()])?.is_empty(),
                    "deleted words stay out"
                );
                add(db, &word("Nguyen"), EntrySource::Manual)?;
                let rejected: i64 =
                    db.query_row("SELECT COUNT(*) FROM dictionary_rejected", [], |row| {
                        row.get(0)
                    })?;
                assert_eq!(rejected, 0, "adding it by hand forgives it");
                Ok(())
            })
            .unwrap();
    }
}
