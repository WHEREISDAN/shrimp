//! Local SQLite storage: the personal dictionary and snippets (phase 2),
//! history and stats later. Only Rust reads and writes it.

mod db;
pub mod dictionary;
pub mod import;
pub mod snippets;

pub use db::{Database, StorageError};
pub use dictionary::{DictionaryEntry, DictionaryInput, EntrySource};
pub use import::SkippedRow;
pub use snippets::{Snippet, SnippetInput, SnippetSaved};
