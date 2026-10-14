//! Snippets: spoken triggers that expand to saved text (SNP-01, SNP-04, SNP-05).

use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::db::{StorageError, now_ms};
use super::import::{MAX_IMPORT_ENTRIES, SkippedRow, check_size, parse_snippet_json, too_many};
use crate::t;

pub const MAX_TRIGGER_CHARS: usize = 60;
pub const MAX_EXPANSION_CHARS: usize = 4000;

/// Mirrors `Snippet` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: i64,
    pub trigger: String,
    /// Markdown subset: bold, italic, lists and links (SNP-03).
    pub expansion: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Mirrors `SnippetInput` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetInput {
    pub trigger: String,
    pub expansion: String,
}

impl SnippetInput {
    pub fn normalized(&self) -> Result<Self, String> {
        let trigger = self.trigger.trim();
        let expansion = self.expansion.trim_end();
        if trigger.is_empty() || expansion.trim().is_empty() {
            return Err(t!("error.snippets.empty"));
        }
        if trigger.chars().count() > MAX_TRIGGER_CHARS {
            return Err(t!("error.snippets.tooLong", max = MAX_TRIGGER_CHARS));
        }
        if !trigger.chars().any(char::is_alphanumeric) {
            return Err(t!("error.snippets.noLetters"));
        }
        if expansion.chars().count() > MAX_EXPANSION_CHARS {
            return Err(t!("error.snippets.expansionTooLong"));
        }
        Ok(Self {
            trigger: trigger.to_owned(),
            expansion: expansion.to_owned(),
        })
    }
}

/// A saved snippet, plus a warning when its trigger resembles another one.
/// Mirrors `SnippetSaved` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetSaved {
    pub snippet: Snippet,
    pub warning: Option<String>,
}

const COLUMNS: &str = "id, trigger, expansion, created_at, updated_at";

fn snippet(row: &Row<'_>) -> rusqlite::Result<Snippet> {
    Ok(Snippet {
        id: row.get(0)?,
        trigger: row.get(1)?,
        expansion: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

pub fn list(db: &Connection) -> Result<Vec<Snippet>, StorageError> {
    let mut statement = db.prepare(&format!(
        "SELECT {COLUMNS} FROM snippets ORDER BY trigger COLLATE NOCASE"
    ))?;
    let snippets = statement
        .query_map([], snippet)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(snippets)
}

pub fn get(db: &Connection, id: i64) -> Result<Snippet, StorageError> {
    db.query_row(
        &format!("SELECT {COLUMNS} FROM snippets WHERE id = ?1"),
        [id],
        snippet,
    )
    .optional()?
    .ok_or(StorageError::NotFound)
}

/// Rejects a trigger that is already a snippet, or a dictionary word or
/// misspelling, in any case (SNP-01).
fn check_unique(db: &Connection, trigger: &str, id: Option<i64>) -> Result<(), StorageError> {
    let snippet: Option<i64> = db
        .query_row(
            "SELECT id FROM snippets WHERE trigger = ?1 COLLATE NOCASE",
            [trigger],
            |row| row.get(0),
        )
        .optional()?;
    if snippet.is_some_and(|other| Some(other) != id) {
        return Err(StorageError::Invalid(t!(
            "error.snippets.duplicate",
            trigger = trigger
        )));
    }
    let word: Option<i64> = db
        .query_row(
            "SELECT id FROM dictionary
             WHERE term = ?1 COLLATE NOCASE OR misspelling = ?1 COLLATE NOCASE",
            [trigger],
            |row| row.get(0),
        )
        .optional()?;
    if word.is_some() {
        return Err(StorageError::Invalid(t!(
            "error.dictionary.duplicate",
            word = trigger
        )));
    }
    Ok(())
}

pub fn add(db: &Connection, input: &SnippetInput) -> Result<SnippetSaved, StorageError> {
    let input = input.normalized().map_err(StorageError::Invalid)?;
    check_unique(db, &input.trigger, None)?;
    let now = now_ms();
    db.execute(
        "INSERT INTO snippets (trigger, expansion, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![input.trigger, input.expansion, now],
    )?;
    saved(db, db.last_insert_rowid())
}

pub fn update(
    db: &Connection,
    id: i64,
    input: &SnippetInput,
) -> Result<SnippetSaved, StorageError> {
    let current = get(db, id)?;
    let input = input.normalized().map_err(StorageError::Invalid)?;
    if input.trigger == current.trigger && input.expansion == current.expansion {
        return Err(StorageError::Invalid(t!("error.snippets.unchanged")));
    }
    check_unique(db, &input.trigger, Some(id))?;
    db.execute(
        "UPDATE snippets SET trigger = ?1, expansion = ?2, updated_at = ?3 WHERE id = ?4",
        params![input.trigger, input.expansion, now_ms(), id],
    )?;
    saved(db, id)
}

fn saved(db: &Connection, id: i64) -> Result<SnippetSaved, StorageError> {
    let snippet = get(db, id)?;
    let others = list(db)?;
    let warning = others
        .iter()
        .filter(|other| other.id != id)
        .find(|other| similar(&snippet.trigger, &other.trigger))
        .map(|other| t!("error.snippets.similar", trigger = other.trigger));
    Ok(SnippetSaved { snippet, warning })
}

pub fn delete(db: &mut Connection, ids: &[i64]) -> Result<usize, StorageError> {
    let transaction = db.transaction()?;
    let mut deleted = 0;
    for id in ids {
        deleted += transaction.execute("DELETE FROM snippets WHERE id = ?1", [id])?;
    }
    transaction.commit()?;
    Ok(deleted)
}

/// Triggers that differ only by case, spacing and punctuation, contain one
/// another, or are one or two typos apart.
fn similar(a: &str, b: &str) -> bool {
    let (a, b) = (letters(a), letters(b));
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let shorter = a.chars().count().min(b.chars().count());
    if a == b || (shorter >= 3 && (a.contains(&b) || b.contains(&a))) {
        return true;
    }
    let allowed = match shorter {
        0..4 => 0,
        4..8 => 1,
        _ => 2,
    };
    edit_distance(&a, &b) <= allowed
}

fn letters(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            current.push(
                (previous[j] + cost)
                    .min(previous[j + 1] + 1)
                    .min(current[j] + 1),
            );
        }
        previous = current;
    }
    previous[b.len()]
}

/// Backslash-escapes the characters the expansion renderer reads as markup
/// (`edit::markdown`), so imported plain text stays literal.
pub fn escape_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '*' | '_' | '[' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// What a JSON import would do. Mirrors `SnippetImportPreview` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub added: Vec<SnippetInput>,
    pub duplicates: Vec<SnippetInput>,
    pub skipped: Vec<SkippedRow>,
}

pub fn preview_import(db: &Connection, json: &str) -> Result<ImportPreview, StorageError> {
    check_size(json).map_err(StorageError::Invalid)?;
    let (rows, mut skipped) = parse_snippet_json(json).map_err(StorageError::Invalid)?;
    if rows.is_empty() && skipped.is_empty() {
        return Err(StorageError::Invalid(t!("error.snippets.emptyImport")));
    }
    let mut seen = HashSet::new();
    let mut added = Vec::new();
    let mut duplicates = Vec::new();
    for (line, row) in rows {
        // Imported text is plain, so markdown characters are kept literally.
        let input = SnippetInput {
            trigger: row.trigger,
            expansion: escape_markdown(&row.expansion),
        };
        let input = match input.normalized() {
            Ok(input) => input,
            Err(reason) => {
                skipped.push(SkippedRow { line, reason });
                continue;
            }
        };
        let fresh = seen.insert(input.trigger.to_lowercase());
        if !fresh || check_unique(db, &input.trigger, None).is_err() {
            duplicates.push(input);
        } else {
            added.push(input);
        }
    }
    if added.len() > MAX_IMPORT_ENTRIES {
        return Err(StorageError::Invalid(too_many()));
    }
    Ok(ImportPreview {
        added,
        duplicates,
        skipped,
    })
}

pub fn import(db: &mut Connection, json: &str) -> Result<usize, StorageError> {
    let preview = preview_import(db, json)?;
    let transaction = db.transaction()?;
    for input in &preview.added {
        add(&transaction, input)?;
    }
    transaction.commit()?;
    Ok(preview.added.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Database, DictionaryInput, EntrySource, dictionary};

    fn input(trigger: &str, expansion: &str) -> SnippetInput {
        SnippetInput {
            trigger: trigger.into(),
            expansion: expansion.into(),
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

    #[test]
    fn creates_and_validates_snippets() {
        with_db(|db| {
            let saved = add(db, &input(" my calendar ", "https://cal.com/dan\n")).unwrap();
            assert_eq!(saved.snippet.trigger, "my calendar");
            assert_eq!(saved.snippet.expansion, "https://cal.com/dan");
            assert_eq!(saved.warning, None);
            assert!(add(db, &input("My Calendar", "x")).is_err());
            assert_eq!(
                add(db, &input("sig", " ")).unwrap_err().to_string(),
                "Please enter a snippet and its expansion"
            );
            assert!(add(db, &input(&"t".repeat(61), "x")).is_err());
            assert_eq!(
                add(db, &input("long", &"x".repeat(4001)))
                    .unwrap_err()
                    .to_string(),
                "This expansion is too long. Please shorten it and try again."
            );
            assert!(add(db, &input("max", &"x".repeat(4000))).is_ok());
        });
    }

    #[test]
    fn a_dictionary_word_cannot_be_a_trigger() {
        with_db(|db| {
            let word = DictionaryInput {
                term: "Shrimp".into(),
                misspelling: None,
            };
            dictionary::add(db, &word, EntrySource::Manual).unwrap();
            let correction = DictionaryInput {
                term: "Kubernetes".into(),
                misspelling: Some("cooper netties".into()),
            };
            dictionary::add(db, &correction, EntrySource::Manual).unwrap();
            let rejected = |trigger: &str| add(db, &input(trigger, "x")).unwrap_err().to_string();
            assert_eq!(
                rejected("Shrimp"),
                "\"Shrimp\" already exists in your personal dictionary"
            );
            assert_eq!(
                rejected(" shrimp "),
                "\"shrimp\" already exists in your personal dictionary"
            );
            assert!(add(db, &input("kubernetes", "x")).is_err());
            assert!(add(db, &input("Cooper Netties", "x")).is_err());
            assert!(add(db, &input("shrimp fried rice", "x")).is_ok());
        });
    }

    #[test]
    fn similar_triggers_save_with_a_warning() {
        with_db(|db| {
            add(db, &input("my calendar", "a")).unwrap();
            let saved = add(db, &input("my calendr", "b")).unwrap();
            assert!(saved.warning.unwrap().contains("\"my calendar\""));
            assert!(add(db, &input("sig", "c")).unwrap().warning.is_none());
        });
        assert!(similar("E.g. sig", "eg sig"));
        assert!(!similar("sig", "big"));
        assert!(similar("address", "home address"));
    }

    #[test]
    fn edits_and_deletes() {
        with_db(|db| {
            let saved = add(db, &input("sig", "Best")).unwrap();
            let id = saved.snippet.id;
            assert_eq!(
                update(db, id, &input("sig", "Best"))
                    .unwrap_err()
                    .to_string(),
                "No changes made"
            );
            let edited = update(db, id, &input("sig", "Thanks")).unwrap();
            assert_eq!(edited.snippet.expansion, "Thanks");
            assert_eq!(delete(db, &[id]).unwrap(), 1);
        });
    }

    #[test]
    fn json_import_previews_duplicates_and_escapes_markdown() {
        with_db(|db| {
            add(db, &input("sig", "Best")).unwrap();
            let json = r#"[{"name":"SIG","text":"x"},{"name":"addr","text":"1 *Main* St"},{"name":"addr","text":"y"},{"name":"big","text":"z"}]"#;
            let preview = preview_import(db, json).unwrap();
            assert_eq!(
                preview.added,
                [input("addr", "1 \\*Main\\* St"), input("big", "z")]
            );
            assert_eq!(preview.duplicates.len(), 2);
            assert_eq!(import(db, json).unwrap(), 2);
            assert_eq!(
                preview_import(db, "[]").unwrap_err().to_string(),
                "No snippets found in file"
            );
        });
    }
}
