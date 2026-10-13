//! The personal dictionary and snippets pages.

use std::sync::Arc;

use rusqlite::Connection;
use tauri::State;

use super::CommandError;
use crate::storage::{
    Database, DictionaryEntry, DictionaryInput, EntrySource, Snippet, SnippetInput, SnippetSaved,
    StorageError, dictionary, snippets,
};
use crate::t;

async fn run<T: Send + 'static>(
    database: &Arc<Database>,
    work: impl FnOnce(&mut Connection) -> Result<T, StorageError> + Send + 'static,
) -> Result<T, CommandError> {
    let database = Arc::clone(database);
    tauri::async_runtime::spawn_blocking(move || database.with(work))
        .await
        .map_err(|err| CommandError::new("storage", err))?
        .map_err(|error| match error {
            StorageError::Invalid(message) => CommandError::new("invalid", message),
            StorageError::NotFound => CommandError::new("notFound", t!("error.library.notFound")),
            error => CommandError::new("storage", error),
        })
}

#[tauri::command]
pub async fn list_dictionary(
    database: State<'_, Arc<Database>>,
) -> Result<Vec<DictionaryEntry>, CommandError> {
    run(&database, |db| dictionary::list(db)).await
}

#[tauri::command]
pub async fn add_dictionary_entry(
    input: DictionaryInput,
    database: State<'_, Arc<Database>>,
) -> Result<DictionaryEntry, CommandError> {
    run(&database, move |db| {
        dictionary::add(db, &input, EntrySource::Manual)
    })
    .await
}

#[tauri::command]
pub async fn update_dictionary_entry(
    id: i64,
    input: DictionaryInput,
    database: State<'_, Arc<Database>>,
) -> Result<DictionaryEntry, CommandError> {
    run(&database, move |db| {
        dictionary::update(db, id, &input, None)
    })
    .await
}

#[tauri::command]
pub async fn set_dictionary_pinned(
    id: i64,
    pinned: bool,
    database: State<'_, Arc<Database>>,
) -> Result<DictionaryEntry, CommandError> {
    run(&database, move |db| dictionary::set_pinned(db, id, pinned)).await
}

#[tauri::command]
pub async fn delete_dictionary_entries(
    ids: Vec<i64>,
    database: State<'_, Arc<Database>>,
) -> Result<usize, CommandError> {
    run(&database, move |db| dictionary::delete(db, &ids)).await
}

#[tauri::command]
pub async fn preview_dictionary_import(
    csv: String,
    database: State<'_, Arc<Database>>,
) -> Result<dictionary::ImportPreview, CommandError> {
    run(&database, move |db| dictionary::preview_import(db, &csv)).await
}

#[tauri::command]
pub async fn import_dictionary(
    csv: String,
    database: State<'_, Arc<Database>>,
) -> Result<usize, CommandError> {
    run(&database, move |db| dictionary::import(db, &csv)).await
}

#[tauri::command]
pub async fn list_snippets(
    database: State<'_, Arc<Database>>,
) -> Result<Vec<Snippet>, CommandError> {
    run(&database, |db| snippets::list(db)).await
}

#[tauri::command]
pub async fn add_snippet(
    input: SnippetInput,
    database: State<'_, Arc<Database>>,
) -> Result<SnippetSaved, CommandError> {
    run(&database, move |db| snippets::add(db, &input)).await
}

#[tauri::command]
pub async fn update_snippet(
    id: i64,
    input: SnippetInput,
    database: State<'_, Arc<Database>>,
) -> Result<SnippetSaved, CommandError> {
    run(&database, move |db| snippets::update(db, id, &input)).await
}

#[tauri::command]
pub async fn delete_snippets(
    ids: Vec<i64>,
    database: State<'_, Arc<Database>>,
) -> Result<usize, CommandError> {
    run(&database, move |db| snippets::delete(db, &ids)).await
}

#[tauri::command]
pub async fn preview_snippet_import(
    json: String,
    database: State<'_, Arc<Database>>,
) -> Result<snippets::ImportPreview, CommandError> {
    run(&database, move |db| snippets::preview_import(db, &json)).await
}

#[tauri::command]
pub async fn import_snippets(
    json: String,
    database: State<'_, Arc<Database>>,
) -> Result<usize, CommandError> {
    run(&database, move |db| snippets::import(db, &json)).await
}
