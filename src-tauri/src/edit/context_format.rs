//! Casing and spacing from the text around the cursor (FMT-11), on the
//! device, with or without context awareness: a dictation that continues a
//! sentence starts lowercase and gets a leading space, loses its ending
//! punctuation when the sentence goes on after the cursor, and Chinese and
//! Japanese are joined without spaces.

use crate::context::Surroundings;

const WEEKDAYS_AND_MONTHS: [&str; 19] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
    "January",
    "February",
    "March",
    "April",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
    "English",
];

/// Fits `text` between `around.before` and `around.after`, replacing
/// `around.selected`. `protected` words keep their capital (dictionary terms
/// and names on screen).
pub fn fit(text: &str, around: Surroundings<'_>, protected: &[String]) -> String {
    let body = text.trim();
    if body.is_empty() {
        return text.to_owned();
    }
    let mut body = body.to_owned();
    let selected = around.selected;

    let lowercase_selection = selected
        .trim_start()
        .chars()
        .next()
        .is_some_and(char::is_lowercase);
    if (continues_sentence(around.before) || lowercase_selection)
        && !keeps_capital(&body, protected)
    {
        body = lowercase_first(&body);
    }

    if sentence_goes_on(around.after) {
        body = body.trim_end_matches(['.', '!', '?', '…']).to_owned();
    }
    if let Some(mark) = selected
        .trim_end()
        .chars()
        .last()
        .filter(|c| matches!(c, ',' | '?' | '!'))
    {
        body = body.trim_end_matches(['.', ',', '!', '?', '…']).to_owned();
        body.push(mark);
    }
    spaced(&body, around)
}

/// Only the spaces around `text`, for unformatted dictations (FMT-15):
/// its casing and punctuation stay as they are.
pub fn space(text: &str, around: Surroundings<'_>) -> String {
    let body = text.trim();
    if body.is_empty() {
        return text.to_owned();
    }
    spaced(body, around)
}

/// A leading space when the text before needs one, a trailing one when a
/// word follows, or the selection's own spaces.
fn spaced(body: &str, around: Surroundings<'_>) -> String {
    let selected = around.selected;
    let first = body.chars().next().unwrap_or(' ');
    let lead = if selected.is_empty() {
        needs_space_before(around.before, first)
    } else {
        selected.starts_with(' ') && !needs_no_space(around.before)
    };
    let last = body.chars().last().unwrap_or(' ');
    let trail = if selected.is_empty() {
        around
            .after
            .chars()
            .next()
            .is_some_and(|next| next.is_alphanumeric() && !is_cjk(next) && !is_cjk(last))
    } else {
        selected.ends_with(' ')
    };
    format!(
        "{}{body}{}",
        if lead { " " } else { "" },
        if trail { " " } else { "" }
    )
}

/// The text before the cursor stops mid-sentence.
fn continues_sentence(before: &str) -> bool {
    let line = before.trim_end_matches([' ', '\t']);
    let Some(last) = line
        .chars()
        .rev()
        .find(|c| !matches!(c, '"' | '\'' | '”' | '’' | ')' | ']' | '*' | '_'))
    else {
        return false;
    };
    !(last == '\n' || is_cjk(last) || matches!(last, '.' | '!' | '?' | '…' | ':' | '•'))
}

/// The text after the cursor continues the sentence (a lowercase word or a digit).
fn sentence_goes_on(after: &str) -> bool {
    after
        .trim_start_matches([' ', '\t'])
        .chars()
        .next()
        .is_some_and(|next| next.is_lowercase() || next.is_ascii_digit())
}

fn first_word(text: &str) -> &str {
    let start = text
        .find(|c: char| c.is_alphanumeric())
        .unwrap_or(text.len());
    let rest = &text[start..];
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’' || c == '-'))
        .unwrap_or(rest.len());
    &rest[..end]
}

/// "I", acronyms, names and dictionary terms keep their capital.
fn keeps_capital(text: &str, protected: &[String]) -> bool {
    let word = first_word(text);
    if !word.starts_with(char::is_uppercase) {
        return true;
    }
    let base = word.split(['\'', '’']).next().unwrap_or(word);
    base == "I"
        || word.chars().filter(|c| c.is_uppercase()).count() > 1
        || word.chars().any(|c| c.is_ascii_digit())
        || WEEKDAYS_AND_MONTHS.contains(&base)
        || protected.iter().any(|term| {
            term.split_whitespace()
                .next()
                .is_some_and(|first| first == word || first == base)
        })
}

fn lowercase_first(text: &str) -> String {
    let Some(index) = text.find(char::is_alphabetic) else {
        return text.to_owned();
    };
    let c = text[index..].chars().next().unwrap_or_default();
    format!(
        "{}{}{}",
        &text[..index],
        c.to_lowercase(),
        &text[index + c.len_utf8()..]
    )
}

fn needs_space_before(before: &str, first: char) -> bool {
    let Some(last) = before.chars().last() else {
        return false;
    };
    if last.is_whitespace() || is_cjk(last) || is_cjk(first) {
        return false;
    }
    if matches!(first, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}') {
        return false;
    }
    !needs_no_space(before)
}

/// The text before ends with an opening bracket or quote, or a joiner.
fn needs_no_space(before: &str) -> bool {
    let mut chars = before.chars().rev();
    match chars.next() {
        None => true,
        Some(c) if c.is_whitespace() => true,
        Some('(' | '[' | '{' | '“' | '‘' | '/' | '@' | '#' | '-' | '_') => true,
        // A straight quote opens when it follows a space or starts the text.
        Some('"' | '\'') => chars.next().is_none_or(char::is_whitespace),
        Some(_) => false,
    }
}

/// Chinese and Japanese characters, which are written without spaces.
fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3000..=0x303F | 0x3040..=0x309F | 0x30A0..=0x30FF | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at<'a>(before: &'a str, after: &'a str) -> Surroundings<'a> {
        Surroundings {
            before,
            after,
            selected: "",
        }
    }

    #[test]
    fn continues_a_sentence_in_lowercase_with_a_space() {
        assert_eq!(
            fit("Tomorrow at noon.", at("we should talk about it", ""), &[]),
            " tomorrow at noon."
        );
        assert_eq!(
            fit("Grocery.", at("I went to the", " store yesterday"), &[]),
            " grocery"
        );
        assert_eq!(fit("Hello.", at("", "world"), &[]), "Hello ");
    }

    #[test]
    fn a_new_sentence_keeps_its_capital() {
        assert_eq!(fit("See you.", at("Done. ", ""), &[]), "See you.");
        assert_eq!(fit("See you.", at("Done.", ""), &[]), " See you.");
        assert_eq!(fit("See you.", at("Done.\n", ""), &[]), "See you.");
        assert_eq!(fit("See you.", at("", ""), &[]), "See you.");
        assert_eq!(fit("Next point.", at("Agenda:", ""), &[]), " Next point.");
    }

    #[test]
    fn names_acronyms_and_i_stay_capitalized() {
        let protected = vec!["Siobhan".to_owned()];
        assert_eq!(
            fit("Siobhan agreed.", at("and", ""), &protected),
            " Siobhan agreed."
        );
        assert_eq!(fit("I left.", at("and then", ""), &[]), " I left.");
        assert_eq!(fit("I'm in.", at("and", ""), &[]), " I'm in.");
        assert_eq!(fit("NASA called.", at("then", ""), &[]), " NASA called.");
        assert_eq!(fit("Monday works.", at("maybe", ""), &[]), " Monday works.");
    }

    #[test]
    fn selections_keep_case_spaces_and_final_marks() {
        let around = Surroundings {
            before: "It was ",
            after: " and fun",
            selected: "fast,",
        };
        assert_eq!(fit("Quick.", around, &[]), "quick,");
        let around = Surroundings {
            before: "It was",
            after: "and fun",
            selected: " fast ",
        };
        assert_eq!(fit("Quick.", around, &[]), " quick ");
    }

    #[test]
    fn chinese_and_japanese_join_without_spaces() {
        assert_eq!(fit("苹果。", at("我喜欢", ""), &[]), "苹果。");
        assert_eq!(fit("Tokyo.", at("東京", ""), &[]), "Tokyo.");
        assert_eq!(
            fit("감사합니다.", at("안녕하세요.", ""), &[]),
            " 감사합니다."
        );
    }

    #[test]
    fn spacing_alone_keeps_case_and_punctuation() {
        assert_eq!(
            space("Tomorrow at noon.", at("we should talk about it", "today")),
            " Tomorrow at noon. "
        );
        assert_eq!(space("Hello.", at("", "")), "Hello.");
    }

    #[test]
    fn brackets_and_quotes() {
        assert_eq!(fit("Note.", at("see (", ""), &[]), "note.");
        assert_eq!(fit("Hi.", at("he said \"", ""), &[]), "hi.");
        assert_eq!(fit("Then.", at("he said \"hi\"", ""), &[]), " then.");
    }
}
