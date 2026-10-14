//! Everyday English words, very common first names, globally famous names,
//! weekdays and months, and everyday acronyms. Names on screen and
//! auto-learned dictionary words exclude them.
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

/// Globally famous people and brands, which the speech engine already
/// spells right (DIC-07). A short curated list, not a gazetteer.
const FAMOUS_NAMES: &[&str] = &[
    "airbnb",
    "amazon",
    "aristotle",
    "beethoven",
    "beyonce",
    "beyoncé",
    "bezos",
    "biden",
    "chatgpt",
    "churchill",
    "darwin",
    "disney",
    "einstein",
    "elon",
    "facebook",
    "freud",
    "gandhi",
    "google",
    "instagram",
    "iphone",
    "lincoln",
    "linkedin",
    "macbook",
    "mandela",
    "messi",
    "michelangelo",
    "microsoft",
    "mozart",
    "musk",
    "napoleon",
    "netflix",
    "newton",
    "obama",
    "openai",
    "oprah",
    "paypal",
    "picasso",
    "plato",
    "putin",
    "rihanna",
    "ronaldo",
    "samsung",
    "shakespeare",
    "socrates",
    "spotify",
    "starbucks",
    "tesla",
    "tiktok",
    "trump",
    "whatsapp",
    "youtube",
    "zuckerberg",
];

/// Weekdays and months with their usual abbreviations: parts of dates.
const DATE_WORDS: &[&str] = &[
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "mon",
    "tue",
    "tues",
    "wed",
    "thu",
    "thur",
    "thurs",
    "fri",
    "sat",
    "sun",
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
    "jan",
    "feb",
    "mar",
    "apr",
    "jun",
    "jul",
    "aug",
    "sep",
    "sept",
    "oct",
    "nov",
    "dec",
];

/// Acronyms everyone writes the same way.
const ACRONYMS: &[&str] = &[
    "AI", "AKA", "AM", "API", "ASAP", "ATM", "BTW", "CEO", "CFO", "COO", "CTO", "DIY", "DM", "EOD",
    "ETA", "FAQ", "FYI", "GPS", "HR", "ID", "IDK", "IMO", "IT", "KPI", "LOL", "NDA", "OK", "OMG",
    "PC", "PDF", "PM", "PR", "PS", "QA", "ROI", "RSVP", "TBD", "TBH", "TV", "UK", "URL", "US",
    "USA", "VIP", "VP",
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

pub fn is_famous_name(word: &str) -> bool {
    FAMOUS_NAMES.contains(&word.to_lowercase().as_str())
}

/// A weekday or month, written out or abbreviated.
pub fn is_date_word(word: &str) -> bool {
    DATE_WORDS.contains(&word.to_lowercase().as_str())
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
        assert!(is_famous_name("Einstein"));
        assert!(is_famous_name("YouTube"));
        assert!(!is_famous_name("Nguyen"));
        assert!(is_date_word("Monday"));
        assert!(is_date_word("Sept"));
        assert!(!is_date_word("Mondays"));
    }
}
