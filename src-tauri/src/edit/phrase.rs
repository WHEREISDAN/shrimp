//! Whole-phrase matching shared by dictionary corrections and snippet
//! triggers: case-insensitive, never touching letters, digits or `_`, longest
//! match first. A match followed by `'t` is part of a contraction ("don't")
//! and is skipped; `'s` possessives match.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhraseMatch {
    /// Byte range in the searched text.
    pub start: usize,
    pub end: usize,
    /// Index into the phrase list that matched.
    pub phrase: usize,
}

/// Finds non-overlapping matches, scanning left to right and preferring the
/// longest phrase at each position.
pub fn find(text: &str, phrases: &[&str]) -> Vec<PhraseMatch> {
    let mut order: Vec<usize> = (0..phrases.len())
        .filter(|&i| !phrases[i].trim().is_empty())
        .collect();
    order.sort_by_key(|&i| std::cmp::Reverse(phrases[i].chars().count()));

    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut matches = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let boundary_before = i == 0 || !is_word(chars[i - 1].1);
        if !boundary_before {
            i += 1;
            continue;
        }
        let found = order.iter().find_map(|&index| {
            let end = match_at(&chars, i, phrases[index])?;
            boundary_after(&chars, end).then_some((index, end))
        });
        match found {
            Some((phrase, end)) => {
                let start_byte = chars[i].0;
                let end_byte = chars.get(end).map_or(text.len(), |(byte, _)| *byte);
                matches.push(PhraseMatch {
                    start: start_byte,
                    end: end_byte,
                    phrase,
                });
                i = end;
            }
            None => i += 1,
        }
    }
    matches
}

/// Replaces every match with `replacement(phrase index)`.
pub fn replace(
    text: &str,
    matches: &[PhraseMatch],
    mut replacement: impl FnMut(usize) -> String,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for found in matches {
        out.push_str(&text[last..found.start]);
        out.push_str(&replacement(found.phrase));
        last = found.end;
    }
    out.push_str(&text[last..]);
    out
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The char index just past `phrase` when it matches at `start`. Any run of
/// whitespace in the phrase matches any run of whitespace in the text.
fn match_at(chars: &[(usize, char)], start: usize, phrase: &str) -> Option<usize> {
    let mut i = start;
    let mut pattern = phrase.trim().chars().peekable();
    while let Some(p) = pattern.next() {
        if p.is_whitespace() {
            while pattern.peek().is_some_and(|c| c.is_whitespace()) {
                pattern.next();
            }
            if !chars.get(i)?.1.is_whitespace() {
                return None;
            }
            while chars.get(i).is_some_and(|(_, c)| c.is_whitespace()) {
                i += 1;
            }
            continue;
        }
        let (_, c) = *chars.get(i)?;
        if c != p && !c.to_lowercase().eq(p.to_lowercase()) {
            return None;
        }
        i += 1;
    }
    Some(i)
}

fn boundary_after(chars: &[(usize, char)], end: usize) -> bool {
    let Some(&(_, next)) = chars.get(end) else {
        return true;
    };
    if is_word(next) {
        return false;
    }
    let contraction = matches!(next, '\'' | '’')
        && chars
            .get(end + 1)
            .is_some_and(|(_, c)| c.eq_ignore_ascii_case(&'t'));
    !contraction
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(text: &str, phrases: &[&str]) -> Vec<String> {
        find(text, phrases)
            .iter()
            .map(|m| text[m.start..m.end].to_owned())
            .collect()
    }

    #[test]
    fn matches_whole_words_ignoring_case() {
        assert_eq!(
            found("Jira and JIRA, not jiras", &["jira"]),
            ["Jira", "JIRA"]
        );
        assert_eq!(found("snake_jira", &["jira"]), Vec::<String>::new());
        assert_eq!(found("jira2", &["jira"]), Vec::<String>::new());
    }

    #[test]
    fn prefers_the_longest_phrase() {
        let phrases = ["my", "my calendar link", "my calendar"];
        let matches = find("send my calendar link now", &phrases);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].phrase, 1);
    }

    #[test]
    fn skips_contractions_but_matches_possessives() {
        assert_eq!(found("don't call Don", &["don"]), ["Don"]);
        assert_eq!(found("John's car", &["john"]), ["John"]);
        assert_eq!(found("can’t", &["can"]), Vec::<String>::new());
    }

    #[test]
    fn whitespace_runs_match() {
        assert_eq!(found("my  calendar", &["my calendar"]), ["my  calendar"]);
    }

    #[test]
    fn internal_punctuation_must_match() {
        assert_eq!(found("use e.g. sig here", &["e.g. sig"]), ["e.g. sig"]);
        assert_eq!(
            found("use eg sig here", &["e.g. sig"]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn replaces_matches() {
        let text = "ask jira about jira";
        let matches = find(text, &["jira"]);
        assert_eq!(
            replace(text, &matches, |_| "Jira".into()),
            "ask Jira about Jira"
        );
    }
}
