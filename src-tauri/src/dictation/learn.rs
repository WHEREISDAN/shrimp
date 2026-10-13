//! Dictionary auto-learn (DIC-07): after a dictation is pasted, watch the
//! field for a while; a word of the dictation that the user respells (and
//! leaves alone for a moment) is added to the dictionary, unless it is a
//! path, a URL, code, a number, an emoji, an everyday word or a word the
//! user deleted from the dictionary before.

use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use super::state::DictationUi;
use crate::context::words;
use crate::edit::Editor;
use crate::notify::{Notice, NoticeAction};
use crate::platform::{ContextRequest, FocusInspector};

const TEXT_LIMIT: usize = 40_000;
const MAX_WORD_CHARS: usize = 40;

/// Where a dictation was pasted, for the watcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearnTarget {
    pub field_id: u64,
    pub inserted: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LearnTiming {
    /// Wait for the app to take the paste before reading the field.
    pub settle: Duration,
    pub poll: Duration,
    pub watch_for: Duration,
}

impl Default for LearnTiming {
    fn default() -> Self {
        Self {
            settle: Duration::from_millis(400),
            poll: Duration::from_secs(1),
            watch_for: Duration::from_secs(60),
        }
    }
}

/// Words the user replaced inside `inserted` (a byte range of `before`):
/// one to three words replaced by one, or as many words for as many.
pub fn respellings(before: &str, after: &str, inserted: &Range<usize>) -> Vec<(String, String)> {
    let prefix = before
        .char_indices()
        .zip(after.chars())
        .find(|((_, a), b)| a != b)
        .map_or(before.len().min(after.len()), |((index, _), _)| index);
    let suffix = before[prefix..]
        .chars()
        .rev()
        .zip(after[prefix..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum::<usize>();
    let word_char = |c: char| c.is_alphanumeric() || c == '\'' || c == '’' || c == '-';
    let start = before[..prefix]
        .char_indices()
        .rev()
        .take_while(|(_, c)| word_char(*c))
        .last()
        .map_or(prefix, |(index, _)| index);
    // Extend the changed span to whole words, but only from inside a word.
    let tail = |text: &str| {
        let end = text.len() - suffix;
        if !text[..end].ends_with(word_char) {
            return end;
        }
        let more: usize = text[end..]
            .chars()
            .take_while(|c| word_char(*c))
            .map(char::len_utf8)
            .sum();
        end + more
    };
    let (old_end, new_end) = (tail(before), tail(after));
    if old_end <= start || old_end <= inserted.start || start >= inserted.end {
        return Vec::new();
    }
    let split = |text: &str| -> Vec<String> {
        text.split(|c: char| !word_char(c))
            .map(|word| word.trim_matches(['\'', '’', '-']).to_owned())
            .filter(|word| !word.is_empty())
            .collect()
    };
    let old = split(&before[start..old_end]);
    let new = split(&after[start..new_end.max(start)]);
    match (old.len(), new.len()) {
        (1..=3, 1) => vec![(old.join(" "), new[0].clone())],
        (count, same) if count == same && count <= 5 => old
            .into_iter()
            .zip(new)
            .filter(|(old, new)| old != new)
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether `new`, written in place of `old`, is worth learning.
pub fn learnable(old: &str, new: &str) -> bool {
    let chars = new.chars().count();
    let letters_only = new
        .chars()
        .all(|c| c.is_alphabetic() || c == '\'' || c == '’' || c == '-');
    if !(2..=MAX_WORD_CHARS).contains(&chars) || !letters_only {
        return false;
    }
    if old.eq_ignore_ascii_case(new) {
        // A casing fix teaches the casing of an unusual word ("tauri" → "Tauri").
        return new.chars().any(char::is_uppercase) && !words::is_common(new);
    }
    !(words::is_common(new) || words::is_common_first_name(new) || words::is_everyday_acronym(new))
}

/// Watches the field until the next dictation (`generation` moves on), the
/// focus moves to another field, or the watch time runs out.
pub async fn watch(
    target: LearnTarget,
    focus: Arc<dyn FocusInspector>,
    editor: Arc<Editor>,
    ui: Arc<dyn DictationUi>,
    generation: Arc<AtomicU64>,
    timing: LearnTiming,
) {
    let mine = generation.load(Ordering::SeqCst);
    let current = |generation: &AtomicU64| generation.load(Ordering::SeqCst) == mine;
    tokio::time::sleep(timing.settle).await;
    let Some(mut baseline) = read(&focus, target.field_id).await else {
        return;
    };
    let Some(start) = baseline.rfind(&target.inserted) else {
        return;
    };
    let mut inserted = start..start + target.inserted.len();
    let mut last = baseline.clone();
    let deadline = Instant::now() + timing.watch_for;
    while Instant::now() < deadline && current(&generation) {
        tokio::time::sleep(timing.poll).await;
        let Some(text) = read(&focus, target.field_id).await else {
            return;
        };
        if text != last {
            last = text;
            continue;
        }
        if text == baseline {
            continue;
        }
        // Unchanged since the last look: the user has finished this edit.
        let words: Vec<String> = respellings(&baseline, &text, &inserted)
            .into_iter()
            .filter(|(old, new)| learnable(old, new))
            .map(|(_, new)| new)
            .collect();
        if !words.is_empty() {
            let learned = editor.learn(words).await;
            if !learned.is_empty() {
                ui.notice(added(&learned).with_action(NoticeAction::OpenDictionary));
            }
        }
        let grown = text.len() as isize - baseline.len() as isize;
        inserted =
            inserted.start..(inserted.end as isize + grown).max(inserted.start as isize) as usize;
        baseline = text;
    }
}

async fn read(focus: &Arc<dyn FocusInspector>, field_id: u64) -> Option<String> {
    let focus = Arc::clone(focus);
    let request = ContextRequest {
        text_limit: TEXT_LIMIT,
        ..ContextRequest::default()
    };
    let field = tokio::task::spawn_blocking(move || focus.read_field(&request))
        .await
        .ok()
        .flatten()?;
    (field.field_id == field_id).then_some(field.text).flatten()
}

fn added(words: &[String]) -> Notice {
    let quoted: Vec<String> = words.iter().map(|word| format!("“{word}”")).collect();
    let list = match quoted.as_slice() {
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        [] => String::new(),
    };
    Notice::info(format!("Added {list} to your dictionary"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(text: &str, part: &str) -> Range<usize> {
        let start = text.find(part).unwrap();
        start..start + part.len()
    }

    #[test]
    fn finds_a_respelled_word_inside_the_dictation() {
        let before = "Notes. Please ask when to review the draft.";
        let inserted = range(before, "Please ask when to review the draft.");
        assert_eq!(
            respellings(
                before,
                "Notes. Please ask Nguyen to review the draft.",
                &inserted
            ),
            [("when".to_owned(), "Nguyen".to_owned())]
        );
        assert_eq!(
            respellings(
                "we moved onto cooper netties today",
                "we moved onto Kubernetes today",
                &(0..34)
            ),
            [("cooper netties".to_owned(), "Kubernetes".to_owned())]
        );
    }

    #[test]
    fn edits_outside_the_dictation_or_deletions_teach_nothing() {
        let before = "Old text. New dictation here.";
        let inserted = range(before, "New dictation here.");
        assert!(respellings(before, "Olde text. New dictation here.", &inserted).is_empty());
        assert!(respellings(before, "Old text. New here.", &inserted).is_empty());
        assert!(respellings(before, before, &inserted).is_empty());
    }

    #[test]
    fn only_unusual_words_are_learned() {
        assert!(learnable("when", "Nguyen"));
        assert!(learnable("tauri", "Tauri"));
        assert!(!learnable("there", "their"), "everyday word");
        assert!(!learnable("x", "src/main.rs"), "path");
        assert!(!learnable("x", "example.com"), "domain");
        assert!(!learnable("x", "user_id"), "code");
        assert!(!learnable("x", "v2"), "number");
        assert!(!learnable("x", "🦐"), "emoji");
        assert!(!learnable("x", "John"), "common first name");
        assert!(!learnable("x", "ASAP"), "everyday acronym");
        assert!(!learnable("the", "The"), "casing of an everyday word");
    }

    #[test]
    fn the_notice_lists_the_words() {
        assert_eq!(
            added(&["Nguyen".into()]).message,
            "Added “Nguyen” to your dictionary"
        );
        assert_eq!(
            added(&["Nguyen".into(), "Siobhan".into(), "Tauri".into()]).message,
            "Added “Nguyen”, “Siobhan” and “Tauri” to your dictionary"
        );
    }
}
