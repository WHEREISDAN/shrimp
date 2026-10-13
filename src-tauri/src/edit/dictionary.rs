//! Dictionary corrections (DIC-03), vocabulary hints for STT and the LLM
//! (DIC-02, DIC-04) and usage counting.

use std::collections::BTreeSet;

use super::phrase;
use crate::storage::DictionaryEntry;

/// Whisper's initial prompt holds about 224 tokens; stay well inside it.
const MAX_VOCABULARY_CHARS: usize = 600;

/// Replaces misspellings with their saved spelling: whole words, any case,
/// longest first. Returns the text and the ids of the corrections applied.
pub fn correct(text: &str, entries: &[DictionaryEntry]) -> (String, Vec<i64>) {
    let corrections: Vec<&DictionaryEntry> = entries
        .iter()
        .filter(|entry| entry.misspelling.is_some())
        .collect();
    let phrases: Vec<&str> = corrections
        .iter()
        .filter_map(|entry| entry.misspelling.as_deref())
        .collect();
    let matches = phrase::find(text, &phrases);
    let used = matches.iter().map(|m| corrections[m.phrase].id).collect();
    let corrected = phrase::replace(text, &matches, |index| corrections[index].term.clone());
    (corrected, used)
}

/// Terms that bias recognition and that the LLM must spell as saved, pinned
/// and most used first (the order `storage::dictionary::list` returns).
pub fn vocabulary(entries: &[DictionaryEntry]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut total = 0;
    let mut terms = Vec::new();
    for entry in entries {
        let term = entry.term.trim();
        if !seen.insert(term.to_lowercase()) {
            continue;
        }
        total += term.len() + 2;
        if total > MAX_VOCABULARY_CHARS {
            break;
        }
        terms.push(term.to_owned());
    }
    terms
}

/// Entries whose spelling appears in the final text, for usage counts.
pub fn used_terms(text: &str, entries: &[DictionaryEntry]) -> Vec<i64> {
    let phrases: Vec<&str> = entries.iter().map(|entry| entry.term.as_str()).collect();
    let mut ids: Vec<i64> = phrase::find(text, &phrases)
        .iter()
        .map(|m| entries[m.phrase].id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::storage::EntrySource;

    pub fn entry(id: i64, term: &str, misspelling: Option<&str>) -> DictionaryEntry {
        DictionaryEntry {
            id,
            term: term.into(),
            misspelling: misspelling.map(Into::into),
            source: EntrySource::Manual,
            pinned: false,
            uses: 0,
            created_at: 0,
        }
    }

    #[test]
    fn corrections_keep_the_saved_casing() {
        let entries = [
            entry(1, "Dawn", Some("Don")),
            entry(2, "Bob", Some("john")),
            entry(3, "Kubernetes", Some("cooper netties")),
            entry(4, "Shrimp", None),
        ];
        let (text, used) = correct("don't tell DON that John's cooper netties broke", &entries);
        assert_eq!(text, "don't tell Dawn that Bob's Kubernetes broke");
        assert_eq!(used, [1, 2, 3]);
    }

    #[test]
    fn the_longest_correction_wins() {
        let entries = [
            entry(1, "A", Some("new york")),
            entry(2, "B", Some("new york city")),
        ];
        assert_eq!(correct("in new york city", &entries).0, "in B");
    }

    #[test]
    fn vocabulary_keeps_order_and_skips_repeats() {
        let entries = [
            entry(1, "Tauri", None),
            entry(2, "Jira", Some("jira")),
            entry(3, "tauri", None),
        ];
        assert_eq!(vocabulary(&entries), ["Tauri", "Jira"]);
        let many: Vec<DictionaryEntry> = (0..200)
            .map(|i| entry(i, &format!("Term{i:03}"), None))
            .collect();
        assert!(vocabulary(&many).join(", ").len() <= MAX_VOCABULARY_CHARS);
    }

    #[test]
    fn counts_terms_in_the_final_text() {
        let entries = [entry(1, "Tauri", None), entry(2, "Jira", Some("jura"))];
        assert_eq!(used_terms("Tauri, Tauri and Jira", &entries), [1, 2]);
    }
}
