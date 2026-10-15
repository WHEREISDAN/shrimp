//! Per-app transcription formats (FMT-15): automatic formatting, the
//! transcript unformatted, capitals only for proper nouns, or all
//! lowercase. The casing rules run on the edited text; snippet expansions
//! keep their own text, and URLs, paths, emails and code keep their case.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::markdown::{SNIPPET_END, SNIPPET_START};
use crate::context::words;

/// Mirrors `TranscriptFormat` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TranscriptFormat {
    /// The AI edit, styles and context casing, as in every other app.
    #[default]
    Automatic,
    /// The transcript as heard: no AI edit, spoken punctuation, style or
    /// context casing. Line commands, dictionary corrections, snippets and
    /// "press enter" still apply.
    Unformatted,
    /// Formatted, but only proper nouns keep a capital.
    ProperNouns,
    /// Formatted, then everything lowercase, sentence starts too.
    Lowercase,
}

impl TranscriptFormat {
    /// Proper-noun capitals and all lowercase change the edited text's case.
    pub fn recases(self) -> bool {
        matches!(self, Self::ProperNouns | Self::Lowercase)
    }
}

/// Words that keep their capital under [`TranscriptFormat::ProperNouns`]:
/// dictionary terms, names on screen and the words the speech engine
/// capitalised mid-sentence.
#[derive(Debug, Clone, Default)]
pub struct ProperNouns {
    words: HashSet<String>,
}

impl ProperNouns {
    pub fn new(raw: &str, terms: &[String], names: &[String]) -> Self {
        let listed = terms
            .iter()
            .chain(names)
            .flat_map(|term| term.split_whitespace())
            .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()).to_owned());
        let words = listed.chain(capitalised_mid_sentence(raw)).collect();
        Self { words }
    }

    /// Whether `word`, capitalised, names something: a listed word, a
    /// weekday or month, a common or famous name, or a word that isn't
    /// everyday English.
    fn keeps(&self, word: &str) -> bool {
        let base = word.split(['\'', '’']).next().unwrap_or(word);
        self.words.contains(word)
            || self.words.contains(base)
            || words::is_date_word(base)
            || words::is_common_first_name(base)
            || words::is_famous_name(base)
            || !words::is_common(base)
    }
}

/// Words with a capital first letter that follow other words in a sentence.
fn capitalised_mid_sentence(raw: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut sentence_start = true;
    for token in raw.split_whitespace() {
        let word = token.trim_matches(|c: char| !c.is_alphanumeric());
        if !sentence_start && word.starts_with(char::is_uppercase) && word != "I" {
            found.push(word.to_owned());
        }
        if !word.is_empty() {
            sentence_start = token.ends_with(['.', '!', '?', '…', ':']);
        }
    }
    found
}

/// Applies the casing of `format` outside snippet expansions.
pub fn apply_case(text: &str, format: TranscriptFormat, proper: &ProperNouns) -> String {
    if !format.recases() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(SNIPPET_START) {
        out.push_str(&recase(&rest[..start], format, proper));
        let snippet = &rest[start..];
        let end = snippet
            .find(SNIPPET_END)
            .map_or(snippet.len(), |end| end + SNIPPET_END.len_utf8());
        out.push_str(&snippet[..end]);
        rest = &snippet[end..];
    }
    out.push_str(&recase(rest, format, proper));
    out
}

/// Recases every word of `text`, keeping its whitespace and punctuation.
fn recase(text: &str, format: TranscriptFormat, proper: &ProperNouns) -> String {
    text.split_inclusive(char::is_whitespace)
        .map(|token| {
            if looks_technical(token) {
                return token.to_owned();
            }
            let mut out = String::with_capacity(token.len());
            let mut rest = token;
            while let Some(start) = rest.find(char::is_alphabetic) {
                out.push_str(&rest[..start]);
                rest = &rest[start..];
                let end = rest
                    .find(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’'))
                    .unwrap_or(rest.len());
                out.push_str(&recase_word(&rest[..end], format, proper));
                rest = &rest[end..];
            }
            out.push_str(rest);
            out
        })
        .collect()
}

fn recase_word(word: &str, format: TranscriptFormat, proper: &ProperNouns) -> String {
    if format == TranscriptFormat::Lowercase {
        return word.to_lowercase();
    }
    let mut chars = word.chars();
    let title_case = chars.next().is_some_and(char::is_uppercase)
        && !chars.any(char::is_uppercase)
        && !word.chars().any(|c| c.is_ascii_digit());
    let base = word.split(['\'', '’']).next().unwrap_or(word);
    // "I", acronyms and mixed case (iPhone, McDonald) already say what they are.
    if !title_case || base == "I" || proper.keeps(word) {
        return word.to_owned();
    }
    word.to_lowercase()
}

/// URLs, paths, emails and code keep their case.
fn looks_technical(token: &str) -> bool {
    let token = token.trim_end();
    token.contains("://")
        || token.starts_with("www.")
        || token.contains(['@', '/', '\\', '_', '=', '`'])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proper(raw: &str, terms: &[&str], names: &[&str]) -> ProperNouns {
        let owned = |list: &[&str]| {
            list.iter()
                .map(|item| (*item).to_owned())
                .collect::<Vec<_>>()
        };
        ProperNouns::new(raw, &owned(terms), &owned(names))
    }

    #[test]
    fn automatic_and_unformatted_leave_the_case_alone() {
        let text = "Sounds good. See you Monday.";
        for format in [TranscriptFormat::Automatic, TranscriptFormat::Unformatted] {
            assert_eq!(apply_case(text, format, &ProperNouns::default()), text);
        }
    }

    #[test]
    fn proper_noun_capitals_only() {
        let raw = "sounds good. ask Siobhan and the Kubernetes team. i think so";
        let nouns = proper(raw, &["Shrimp"], &["Kathryn"]);
        let case = |text: &str| apply_case(text, TranscriptFormat::ProperNouns, &nouns);
        assert_eq!(
            case("Sounds good. Ask Siobhan and the Kubernetes team. I think so."),
            "sounds good. ask Siobhan and the Kubernetes team. I think so."
        );
        assert_eq!(
            case("Shrimp works. Kathryn agreed. Monday works. NASA called. I'm in."),
            "Shrimp works. Kathryn agreed. Monday works. NASA called. I'm in."
        );
        assert_eq!(
            case("The iPhone and McDonald's. Nguyen replied. John too."),
            "the iPhone and McDonald's. Nguyen replied. John too."
        );
        assert_eq!(case("- Milk\n- Eggs"), "- milk\n- eggs");
    }

    #[test]
    fn mid_sentence_capitals_from_the_transcript_count_as_names() {
        let nouns = proper("we met with Rose from sales", &[], &[]);
        assert_eq!(
            apply_case(
                "Rose said hi. We met with Rose.",
                TranscriptFormat::ProperNouns,
                &nouns
            ),
            "Rose said hi. we met with Rose."
        );
        let none = proper("Rose said hi.", &[], &[]);
        assert_eq!(
            apply_case("Rose said hi.", TranscriptFormat::ProperNouns, &none),
            "rose said hi.",
            "a sentence start is no evidence"
        );
    }

    #[test]
    fn all_lowercase_covers_sentence_starts_names_and_i() {
        let nouns = proper("Ask Siobhan", &["Shrimp"], &[]);
        assert_eq!(
            apply_case(
                "Ask Siobhan. I think NASA and Shrimp agree.",
                TranscriptFormat::Lowercase,
                &nouns
            ),
            "ask siobhan. i think nasa and shrimp agree."
        );
    }

    #[test]
    fn snippets_urls_and_code_keep_their_case() {
        let text = format!(
            "Thanks. {SNIPPET_START}Best,\nAlex Rivera{SNIPPET_END} See https://Example.com/Path and user_Id."
        );
        assert_eq!(
            apply_case(&text, TranscriptFormat::Lowercase, &ProperNouns::default()),
            format!(
                "thanks. {SNIPPET_START}Best,\nAlex Rivera{SNIPPET_END} see https://Example.com/Path and user_Id."
            )
        );
        assert_eq!(
            apply_case(
                "Mail Alex@Example.com now.",
                TranscriptFormat::ProperNouns,
                &ProperNouns::default()
            ),
            "mail Alex@Example.com now."
        );
    }

    #[test]
    fn serializes_in_camel_case() {
        assert_eq!(
            serde_json::to_value(TranscriptFormat::ProperNouns).unwrap(),
            "properNouns"
        );
        assert_eq!(
            serde_json::from_str::<TranscriptFormat>("\"lowercase\"").unwrap(),
            TranscriptFormat::Lowercase
        );
    }
}
