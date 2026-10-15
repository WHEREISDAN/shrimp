//! Snippet triggers (SNP-02). Spoken alone, a trigger matches ignoring case,
//! punctuation and spacing. Inside a sentence the LLM keeps the trigger word
//! for word, and it is expanded afterwards.

use super::markdown::{SNIPPET_END, SNIPPET_START};
use super::phrase;
use crate::storage::Snippet;

/// The snippet whose trigger is the whole dictation.
pub fn match_alone<'a>(text: &str, snippets: &'a [Snippet]) -> Option<&'a Snippet> {
    let spoken = letters(text);
    if spoken.is_empty() {
        return None;
    }
    snippets
        .iter()
        .find(|snippet| letters(&snippet.trigger) == spoken)
}

fn letters(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The snippets whose triggers occur in the text, in order of appearance.
/// The triggers stay in the text the LLM edits and are expanded afterwards.
pub fn find_triggers(text: &str, snippets: &[Snippet]) -> Vec<usize> {
    let triggers: Vec<&str> = snippets.iter().map(|s| s.trigger.as_str()).collect();
    phrase::find(text, &triggers)
        .iter()
        .map(|m| m.phrase)
        .collect()
}

/// Replaces each trigger with its expansion, marked for rich-text rendering.
/// None unless exactly the `expected` triggers are still there (the model
/// may have reworded one).
pub fn expand(text: &str, expected: &[usize], snippets: &[Snippet]) -> Option<String> {
    let triggers: Vec<&str> = snippets.iter().map(|s| s.trigger.as_str()).collect();
    let matches = phrase::find(text, &triggers);
    let mut found: Vec<usize> = matches.iter().map(|m| m.phrase).collect();
    let mut wanted = expected.to_vec();
    found.sort_unstable();
    wanted.sort_unstable();
    if found != wanted {
        return None;
    }
    Some(phrase::replace(text, &matches, |index| {
        format!("{SNIPPET_START}{}{SNIPPET_END}", snippets[index].expansion)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn snippet(id: i64, trigger: &str, expansion: &str) -> Snippet {
        Snippet {
            id,
            trigger: trigger.into(),
            expansion: expansion.into(),
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn a_trigger_alone_ignores_case_punctuation_and_spacing() {
        let snippets = [snippet(1, "my calendar link", "https://cal.com/alex")];
        assert_eq!(match_alone("My calendar-link.", &snippets).unwrap().id, 1);
        assert_eq!(
            match_alone("  MYCALENDAR LINK!  ", &snippets).unwrap().id,
            1
        );
        assert!(match_alone("my calendar link please", &snippets).is_none());
    }

    #[test]
    fn triggers_in_a_sentence_expand_after_the_edit() {
        let snippets = [
            snippet(1, "sig", "Best,\n**Alex**"),
            snippet(2, "my calendar", "https://cal.com/alex"),
        ];
        let found = find_triggers("Book via My Calendar, thanks. sig", &snippets);
        assert_eq!(found, [1, 0]);
        let expanded = expand("Book via my calendar. Thanks!\nSig", &found, &snippets).unwrap();
        assert_eq!(
            expanded,
            "Book via \u{e000}https://cal.com/alex\u{e001}. Thanks!\n\u{e000}Best,\n**Alex**\u{e001}"
        );
    }

    #[test]
    fn a_reworded_or_repeated_trigger_fails() {
        let snippets = [snippet(1, "sig", "Best")];
        assert!(expand("signature", &[0], &snippets).is_none());
        assert!(expand("sig and sig", &[0], &snippets).is_none());
        assert_eq!(expand("plain", &[], &snippets).as_deref(), Some("plain"));
    }

    #[test]
    fn triggers_do_not_touch_words_or_contractions() {
        let snippets = [snippet(1, "sig", "Best")];
        assert_eq!(find_triggers("signal sig_x sig's", &snippets), [0]);
        assert_eq!(
            expand("signal sig_x sig's", &[0], &snippets).unwrap(),
            "signal sig_x \u{e000}Best\u{e001}'s"
        );
    }
}
