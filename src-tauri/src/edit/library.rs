//! The dictionary and snippets as the edit pipeline reads them.

use crate::storage::{Database, DictionaryEntry, Snippet, dictionary, snippets};

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// Pinned and most used first.
    pub dictionary: Vec<DictionaryEntry>,
    pub snippets: Vec<Snippet>,
}

/// Where dictation reads its words and snippets; tests use a fixed set.
pub trait Library: Send + Sync {
    fn snapshot(&self) -> Snapshot;
    /// Counts one use for each dictionary entry a dictation contained.
    fn record_uses(&self, ids: &[i64]);
    /// Adds words learned from corrections; returns the ones added (DIC-07).
    fn learn(&self, words: &[String]) -> Vec<String>;
}

impl Library for Database {
    fn snapshot(&self) -> Snapshot {
        let read = self.with(|db| Ok((dictionary::list(db)?, snippets::list(db)?)));
        match read {
            Ok((dictionary, snippets)) => Snapshot {
                dictionary,
                snippets,
            },
            Err(error) => {
                log::warn!("could not read the dictionary and snippets: {error}");
                Snapshot::default()
            }
        }
    }

    fn record_uses(&self, ids: &[i64]) {
        if ids.is_empty() {
            return;
        }
        if let Err(error) = self.with(|db| dictionary::record_uses(db, ids)) {
            log::warn!("could not count dictionary uses: {error}");
        }
    }

    fn learn(&self, words: &[String]) -> Vec<String> {
        match self.with(|db| dictionary::learn(db, words)) {
            Ok(learned) => learned.into_iter().map(|entry| entry.term).collect(),
            Err(error) => {
                log::warn!("could not add learned words: {error}");
                Vec::new()
            }
        }
    }
}

/// A fixed library, for tests and evaluation runs; learned words are kept
/// in memory.
#[derive(Default)]
pub struct FixedLibrary(pub Snapshot, pub std::sync::Mutex<Vec<String>>);

impl FixedLibrary {
    pub fn new(snapshot: Snapshot) -> Self {
        Self(snapshot, std::sync::Mutex::default())
    }

    pub fn learned(&self) -> Vec<String> {
        self.1.lock().unwrap_or_else(|err| err.into_inner()).clone()
    }
}

impl Library for FixedLibrary {
    fn snapshot(&self) -> Snapshot {
        self.0.clone()
    }

    fn record_uses(&self, _: &[i64]) {}

    fn learn(&self, words: &[String]) -> Vec<String> {
        let mut learned = self.1.lock().unwrap_or_else(|err| err.into_inner());
        let new: Vec<String> = words
            .iter()
            .filter(|word| !learned.contains(word))
            .cloned()
            .collect();
        learned.extend(new.iter().cloned());
        new
    }
}
