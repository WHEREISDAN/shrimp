//! Terms worth spelling exactly, found in the text around the cursor and on
//! screen: names (CTX-02), code identifiers (DEV-02) and file names (DEV-01).

use std::collections::HashMap;

use super::words;

const MAX_NAMES: usize = 40;
const MAX_IDENTIFIERS: usize = 200;
const MAX_FILE_NAMES: usize = 200;
const MAX_TERM_CHARS: usize = 60;
/// Endings that make a dotted word a domain rather than a file.
const DOMAIN_ENDINGS: [&str; 6] = ["com", "org", "net", "edu", "gov", "www"];

/// Counts terms in order of first appearance and returns the most frequent.
struct Tally {
    counts: HashMap<String, (usize, usize)>,
}

impl Tally {
    fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    fn add(&mut self, term: &str) {
        let order = self.counts.len();
        self.counts.entry(term.to_owned()).or_insert((0, order)).0 += 1;
    }

    fn top(self, limit: usize) -> Vec<String> {
        let mut terms: Vec<(String, (usize, usize))> = self.counts.into_iter().collect();
        terms.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.1.1.cmp(&b.1.1)));
        terms
            .into_iter()
            .take(limit)
            .map(|(term, _)| term)
            .collect()
    }
}

fn words_of(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’' || c == '-'))
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|word| !word.is_empty())
}

/// Capitalized words that are not everyday English: people, places, products.
pub fn names<'a>(texts: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut tally = Tally::new();
    for text in texts {
        for word in words_of(text) {
            if is_name(word) {
                tally.add(word);
            }
        }
    }
    tally.top(MAX_NAMES)
}

fn is_name(word: &str) -> bool {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    let count = word.chars().count();
    if !first.is_uppercase() || !(2..=MAX_TERM_CHARS).contains(&count) {
        return false;
    }
    if word.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    let base = word.split(['\'', '’']).next().unwrap_or(word);
    if base != word && words::is_common(base) {
        return false;
    }
    let inner_capital = chars.any(char::is_uppercase);
    let all_caps = word.chars().all(|c| !c.is_lowercase());
    if all_caps {
        return !words::is_everyday_acronym(word);
    }
    inner_capital || !words::is_common(word)
}

/// camelCase, PascalCase and snake_case identifiers in code.
pub fn identifiers(code: &str) -> Vec<String> {
    let mut tally = Tally::new();
    let tokens = code.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'));
    for token in tokens {
        let token = token.trim_matches('$');
        if is_identifier(token) {
            tally.add(token);
        }
    }
    tally.top(MAX_IDENTIFIERS)
}

fn is_identifier(token: &str) -> bool {
    let len = token.len();
    if !(3..=MAX_TERM_CHARS).contains(&len) || token.starts_with(|c: char| c.is_ascii_digit()) {
        return false;
    }
    let core = token.trim_matches('_');
    if core.contains('_') {
        let worded = core
            .split('_')
            .filter(|part| part.chars().any(|c| c.is_ascii_alphabetic()))
            .count();
        return worded >= 2;
    }
    let bytes = core.as_bytes();
    let humps = bytes
        .windows(2)
        .filter(|pair| pair[0].is_ascii_lowercase() && pair[1].is_ascii_uppercase())
        .count();
    humps >= 1
}

/// File names such as `index.ts`, `my_python_script.py` and `.env`.
pub fn file_names<'a>(texts: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut tally = Tally::new();
    for text in texts {
        for token in text.split(|c: char| c.is_whitespace() || matches!(c, ',' | '(' | ')')) {
            let name = token.rsplit(['/', '\\']).next().unwrap_or(token);
            let name = name.trim_matches(|c: char| matches!(c, '"' | '\'' | '`' | ':' | ';'));
            if is_file_name(name) {
                tally.add(name);
            }
        }
    }
    tally.top(MAX_FILE_NAMES)
}

fn is_file_name(name: &str) -> bool {
    if name.len() < 3 || name.len() > MAX_TERM_CHARS || name.ends_with('.') {
        return false;
    }
    let valid = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.');
    if !name.chars().all(valid) {
        return false;
    }
    if let Some(dotfile) = name.strip_prefix('.') {
        return dotfile.starts_with(|c: char| c.is_ascii_alphabetic());
    }
    let Some((stem, extension)) = name.rsplit_once('.') else {
        return false;
    };
    name.starts_with(|c: char| c.is_ascii_alphabetic())
        && !stem.is_empty()
        && (1..=10).contains(&extension.len())
        && extension.chars().any(|c| c.is_ascii_alphabetic())
        && !DOMAIN_ENDINGS.contains(&extension.to_ascii_lowercase().as_str())
        && !stem.to_ascii_lowercase().starts_with("www")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_names_not_everyday_words() {
        let found = names([
            "Dear Siobhan Nguyen, the Quarterly report is attached.",
            "Cc: Siobhan, NASA, PDF, GitHub. We're done.",
        ]);
        assert_eq!(found[0], "Siobhan");
        assert!(found.contains(&"Nguyen".to_owned()));
        assert!(found.contains(&"NASA".to_owned()));
        assert!(found.contains(&"GitHub".to_owned()));
        assert!(!found.contains(&"Dear".to_owned()));
        assert!(!found.contains(&"Quarterly".to_owned()));
        assert!(!found.contains(&"PDF".to_owned()));
        assert!(!found.contains(&"We're".to_owned()));
    }

    #[test]
    fn finds_identifiers() {
        let found = identifiers(
            "const userId = 42; function fetchUserProfile(userId) { return MAX_RETRY_COUNT + UserStore; } let x_1 = id;",
        );
        assert_eq!(found[0], "userId");
        for expected in ["fetchUserProfile", "MAX_RETRY_COUNT", "UserStore"] {
            assert!(found.contains(&expected.to_owned()), "{expected}");
        }
        assert!(!found.contains(&"const".to_owned()));
        assert!(!found.contains(&"x_1".to_owned()));
    }

    #[test]
    fn finds_file_names() {
        let found = file_names([
            "scratch.ts — ide",
            "/tmp/shrimp-p3/ide/index.ts",
            "my_python_script.py, .env, README, example.com, v1.2, www.site.io",
        ]);
        assert_eq!(
            found,
            ["scratch.ts", "index.ts", "my_python_script.py", ".env"]
        );
    }
}
