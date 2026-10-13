//! Everyday English words, very common first names and everyday acronyms.
//! Names on screen and auto-learned dictionary words exclude them.
//!
//! `common_words.txt` is SCOWL 2020.12.07 (english-words and american-words,
//! levels 10 to 35), lowercase words without apostrophes; see
//! `common_words_license.txt`.

use std::sync::LazyLock;

const COMMON_WORDS: &str = include_str!("common_words.txt");

static SORTED: LazyLock<Vec<&'static str>> = LazyLock::new(|| COMMON_WORDS.lines().collect());

/// Very common first names, which the speech engine already spells right.
const FIRST_NAMES: &[&str] = &[
    "aaron",
    "adam",
    "alex",
    "alice",
    "amanda",
    "amy",
    "andrew",
    "angela",
    "anna",
    "anne",
    "anthony",
    "ashley",
    "barbara",
    "ben",
    "betty",
    "bob",
    "brandon",
    "brian",
    "carol",
    "charles",
    "chris",
    "christopher",
    "daniel",
    "david",
    "deborah",
    "donald",
    "donna",
    "dorothy",
    "edward",
    "elizabeth",
    "emily",
    "emma",
    "eric",
    "frank",
    "gary",
    "george",
    "james",
    "jane",
    "jason",
    "jeff",
    "jeffrey",
    "jennifer",
    "jessica",
    "joe",
    "john",
    "jonathan",
    "joseph",
    "joshua",
    "justin",
    "karen",
    "kate",
    "kevin",
    "kimberly",
    "laura",
    "linda",
    "lisa",
    "margaret",
    "maria",
    "mark",
    "mary",
    "matt",
    "matthew",
    "melissa",
    "michael",
    "michelle",
    "mike",
    "nancy",
    "nicholas",
    "nick",
    "olivia",
    "patricia",
    "paul",
    "peter",
    "rachel",
    "rebecca",
    "richard",
    "robert",
    "ryan",
    "sam",
    "sandra",
    "sarah",
    "scott",
    "sophia",
    "stephanie",
    "steve",
    "steven",
    "susan",
    "thomas",
    "timothy",
    "tom",
    "william",
];

/// Acronyms everyone writes the same way.
const ACRONYMS: &[&str] = &[
    "AI", "AM", "API", "ASAP", "ATM", "BTW", "CEO", "CFO", "CTO", "DIY", "DM", "ETA", "FAQ", "FYI",
    "GPS", "HR", "ID", "IT", "LOL", "OK", "PC", "PDF", "PM", "PR", "PS", "QA", "RSVP", "TBD", "TV",
    "UK", "URL", "US", "USA", "VIP", "VP",
];

/// Whether `word` (any case) is an everyday English word.
pub fn is_common(word: &str) -> bool {
    let lower = word.to_lowercase();
    SORTED.binary_search(&lower.as_str()).is_ok()
}

pub fn is_common_first_name(word: &str) -> bool {
    FIRST_NAMES.contains(&word.to_lowercase().as_str())
}

pub fn is_everyday_acronym(word: &str) -> bool {
    ACRONYMS.contains(&word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_is_sorted_for_binary_search() {
        assert!(SORTED.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(SORTED.len() > 30_000);
    }

    #[test]
    fn knows_everyday_words_but_not_names() {
        assert!(is_common("report"));
        assert!(is_common("Tomorrow"));
        assert!(!is_common("Nguyen"));
        assert!(!is_common("Kubernetes"));
        assert!(is_common_first_name("John"));
        assert!(is_everyday_acronym("PDF"));
        assert!(!is_everyday_acronym("pdf"));
    }
}
