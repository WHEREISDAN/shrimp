//! Local SQLite storage: the personal dictionary and snippets, transcript
//! history, usage stats and daily backups. Only Rust reads and writes it.

pub mod backup;
mod db;
pub mod dictionary;
pub mod history;
pub mod import;
pub mod snippets;
pub mod stats;

pub use db::{Database, StorageError, now_ms};
pub use dictionary::{DictionaryEntry, DictionaryInput, EntrySource};
pub use history::{HistoryEntry, HistoryStatus, NewEntry, Retranscribed};
pub use import::SkippedRow;
pub use snippets::{Snippet, SnippetInput, SnippetSaved};
