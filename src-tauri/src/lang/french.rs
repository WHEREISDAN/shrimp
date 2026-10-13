//! French typography (FMT-14): a narrow no-break space (U+202F) before
//! `;`, `:`, `!` and `?`, replacing any plain or no-break space there.
//! Developer apps are left alone, and so are marks that aren't
//! punctuation: times and ratios (10:30), URLs (https://, ?q=), emoticons.

const NARROW_NO_BREAK_SPACE: char = '\u{202F}';
const MARKS: [char; 4] = [';', ':', '!', '?'];

/// Puts the narrow space before every high punctuation mark, replacing a
/// space typed before it: "Bonjour !" and "Bonjour!" both become
/// "Bonjour\u{202F}!".
pub fn apply(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let spaced: String = chars
        .iter()
        .enumerate()
        .map(|(index, &c)| {
            let before_mark = matches!(c, ' ' | '\u{00A0}')
                && index > 0
                && !chars[index - 1].is_whitespace()
                && chars
                    .get(index + 1)
                    .is_some_and(|next| MARKS.contains(next))
                && ends_phrase(&chars, index + 1);
            if before_mark {
                NARROW_NO_BREAK_SPACE
            } else {
                c
            }
        })
        .collect();
    spaced
        .split_inclusive(char::is_whitespace)
        .map(word)
        .collect()
}

/// One whitespace-separated word, with its trailing whitespace.
fn word(token: &str) -> String {
    let body = token.trim_end_matches(char::is_whitespace);
    let core = body.trim_end_matches(&MARKS[..]);
    if looks_technical(core) {
        // A URL or address keeps its characters; a mark after it is still punctuation.
        let trailing = &body[core.len()..];
        let mut out = core.to_owned();
        if !trailing.is_empty() && core.chars().last().is_some_and(char::is_alphanumeric) {
            out.push(NARROW_NO_BREAK_SPACE);
        }
        out.push_str(trailing);
        out.push_str(&token[body.len()..]);
        return out;
    }
    let chars: Vec<char> = token.chars().collect();
    let mut out = String::with_capacity(token.len() + 6);
    for (index, &c) in chars.iter().enumerate() {
        let previous = index.checked_sub(1).map(|at| chars[at]);
        if MARKS.contains(&c)
            && previous.is_some_and(follows_word)
            && ends_phrase(&chars, index)
            && !digits_around(&chars, index)
        {
            out.push(NARROW_NO_BREAK_SPACE);
        }
        out.push(c);
    }
    out
}

fn follows_word(previous: char) -> bool {
    previous.is_alphanumeric() || matches!(previous, '»' | '"' | '”' | ']' | '\'' | '’')
}

/// The mark at `index` ends a phrase: nothing, a space, another mark or a
/// closing quote follows.
fn ends_phrase(chars: &[char], index: usize) -> bool {
    let mut next = index + 1;
    while chars.get(next).is_some_and(|c| MARKS.contains(c)) {
        next += 1;
    }
    match chars.get(next) {
        None => true,
        Some(&c) => c.is_whitespace() || matches!(c, '»' | '"' | '”'),
    }
}

/// 10:30 and 3:2 are not punctuation.
fn digits_around(chars: &[char], index: usize) -> bool {
    chars[index] == ':'
        && index > 0
        && chars[index - 1].is_ascii_digit()
        && chars.get(index + 1).is_some_and(char::is_ascii_digit)
}

/// URLs, emails, paths and code-like words keep their characters.
fn looks_technical(word: &str) -> bool {
    word.contains("://")
        || word.starts_with("www.")
        || word.contains('@')
        || word.contains('/')
        || word.contains('=')
        || word.contains('_')
}

#[cfg(test)]
mod tests {
    use super::*;

    const N: &str = "\u{202F}";

    #[test]
    fn puts_a_narrow_space_before_high_punctuation() {
        assert_eq!(
            apply("Bonjour ! Pouvez-vous venir ? Voici la liste : pain; lait."),
            format!("Bonjour{N}! Pouvez-vous venir{N}? Voici la liste{N}: pain{N}; lait.")
        );
        assert_eq!(apply("Vraiment ?!"), format!("Vraiment{N}?!"));
        assert_eq!(apply("Vraiment?!"), format!("Vraiment{N}?!"));
        assert_eq!(apply("« Oui ! »"), format!("« Oui{N}! »"));
    }

    #[test]
    fn leaves_times_urls_and_emoticons() {
        let text = "Rendez-vous à 10:30 sur https://exemple.fr/page?x=1 :)";
        assert_eq!(apply(text), text);
        assert_eq!(
            apply("Écris à marie@exemple.fr !"),
            format!("Écris à marie@exemple.fr{N}!")
        );
    }

    #[test]
    fn is_idempotent() {
        let once = apply("Pourquoi ? Parce que !");
        assert_eq!(apply(&once), once);
    }
}
