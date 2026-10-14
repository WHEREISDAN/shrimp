//! Spoken commands applied without the LLM: line breaks always (FMT-06), and
//! punctuation words (FMT-05) when no model edits the text.

use super::phrase;

const NEW_LINE: [&str; 4] = ["new line", "next line", "line break", "skip a line"];
const NEW_PARAGRAPH: [&str; 2] = ["start a new paragraph", "new paragraph"];
/// A command right after one of these is content: "a new line of shoes".
const DETERMINERS: [&str; 17] = [
    "a", "an", "the", "this", "that", "each", "every", "another", "our", "your", "my", "his",
    "her", "their", "its", "any", "no",
];

const PUNCTUATION: [(&str, &str); 11] = [
    ("question mark", "?"),
    ("exclamation point", "!"),
    ("exclamation mark", "!"),
    ("full stop", "."),
    ("period", "."),
    ("comma", ","),
    ("semicolon", ";"),
    ("colon", ":"),
    ("em dash", "—"),
    ("em-dash", "—"),
    ("emdash", "—"),
];

/// Replaces spoken line and paragraph commands with `\n` and `\n\n`.
pub fn line_breaks(text: &str) -> String {
    let phrases: Vec<&str> = NEW_PARAGRAPH
        .iter()
        .chain(NEW_LINE.iter())
        .copied()
        .collect();
    let (commands, breaks): (Vec<(usize, usize)>, Vec<&str>) = phrase::find(text, &phrases)
        .into_iter()
        .filter(|m| !after_determiner(text, m.start) && !next_word_is(text, m.end, "of"))
        .map(|m| {
            let paragraph = m.phrase < NEW_PARAGRAPH.len();
            ((m.start, m.end), if paragraph { "\n\n" } else { "\n" })
        })
        .unzip();
    splice(text, &commands, &breaks, Spacing::Break)
}

/// Turns punctuation words into symbols where they read as commands:
/// "period" only before the end, a line break or a new sentence.
pub fn punctuation(text: &str) -> String {
    let phrases: Vec<&str> = PUNCTUATION.iter().map(|(words, _)| *words).collect();
    let mut commands = Vec::new();
    let mut symbols = Vec::new();
    for found in phrase::find(text, &phrases) {
        let (words, symbol) = PUNCTUATION[found.phrase];
        if after_determiner(text, found.start) {
            continue;
        }
        let ends_sentence = matches!(symbol, "." | "?" | "!");
        let rest = text[found.end..].trim_start_matches([' ', '.', ',']);
        let at_boundary = rest.is_empty()
            || rest.starts_with('\n')
            || rest.chars().next().is_some_and(char::is_uppercase);
        if (ends_sentence || words == "colon") && !at_boundary {
            continue;
        }
        commands.push((found.start, found.end));
        symbols.push(symbol);
    }
    splice(text, &commands, &symbols, Spacing::Punctuation)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Spacing {
    Break,
    Punctuation,
}

/// Replaces each command, absorbing the spaces and the stray `.` or `,` the
/// speech engine put around it, and capitalises the next sentence.
fn splice(text: &str, commands: &[(usize, usize)], symbols: &[&str], spacing: Spacing) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut capitalize = false;
    for (&(start, end), symbol) in commands.iter().zip(symbols) {
        push(&mut out, &text[last..start], &mut capitalize);
        let trimmed_len = out.trim_end_matches([' ', '\t']).len();
        out.truncate(trimmed_len);
        if spacing == Spacing::Punctuation {
            // The engine's own comma or period around a spoken one.
            let without_stray = out.trim_end_matches([',', '.']).len();
            out.truncate(without_stray);
        }
        out.push_str(symbol);
        let rest = &text[end..];
        let skipped = rest.len() - rest.trim_start_matches([' ', '\t', '.', ',']).len();
        last = end + skipped;
        let next = &text[last..];
        match spacing {
            Spacing::Break => capitalize = true,
            Spacing::Punctuation => {
                capitalize = matches!(*symbol, "." | "?" | "!");
                if !next.is_empty() && !next.starts_with('\n') && *symbol != "—" {
                    out.push(' ');
                }
            }
        }
    }
    push(&mut out, &text[last..], &mut capitalize);
    out
}

fn push(out: &mut String, text: &str, capitalize: &mut bool) {
    for c in text.chars() {
        if *capitalize && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            *capitalize = false;
        } else {
            if !c.is_whitespace() {
                *capitalize = false;
            }
            out.push(c);
        }
    }
}

fn previous_word(text: &str, start: usize) -> Option<String> {
    let before = text[..start].trim_end();
    let word_start = before
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphanumeric() || *c == '\'')
        .last()
        .map(|(index, _)| index)?;
    Some(before[word_start..].to_owned())
}

fn after_determiner(text: &str, start: usize) -> bool {
    let separated_by_space = text[..start].ends_with(' ');
    separated_by_space
        && previous_word(text, start)
            .is_some_and(|word| DETERMINERS.contains(&word.to_lowercase().as_str()))
}

fn next_word_is(text: &str, end: usize, word: &str) -> bool {
    text[end..]
        .split_whitespace()
        .next()
        .is_some_and(|next| next.eq_ignore_ascii_case(word))
}

/// The words that end a dictation with Enter (CORE-16).
pub const PRESS_ENTER: &str = "press enter";

/// What a final "press enter" asks for; the phrase is cut off unless `No`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PressEnter {
    #[default]
    No,
    /// The command is on: Enter after the paste (CORE-16).
    Press,
    /// The command is off and this is its first detection in a dictation:
    /// a one-time prompt offers to turn it on (CORE-17).
    Ask,
}

/// Looks for a final "press enter" when the command is on, or, while it is
/// off, when its one-time prompt may still show; returns the text before it.
pub fn press_enter(text: &str, command: bool, may_ask: bool) -> (PressEnter, Option<String>) {
    let suffix = (command || may_ask)
        .then(|| press_enter_suffix(text))
        .flatten();
    let found = match (&suffix, command) {
        (None, _) => PressEnter::No,
        (Some(_), true) => PressEnter::Press,
        (Some(_), false) => PressEnter::Ask,
    };
    (found, suffix)
}

/// Strips a final "press enter" and returns the text before it.
pub fn press_enter_suffix(text: &str) -> Option<String> {
    let trimmed = text
        .trim_end()
        .trim_end_matches(['.', '!', '?', ','])
        .trim_end();
    let cut = trimmed.len().checked_sub(PRESS_ENTER.len())?;
    if !trimmed.is_char_boundary(cut) || !trimmed[cut..].eq_ignore_ascii_case(PRESS_ENTER) {
        return None;
    }
    let before = &trimmed[..cut];
    if before.ends_with(char::is_alphanumeric) {
        return None;
    }
    Some(
        before
            .trim_end()
            .trim_end_matches([',', ';'])
            .trim_end()
            .to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_final_press_enter_is_cut_off() {
        assert_eq!(
            press_enter_suffix("Ship it now, press enter."),
            Some("Ship it now".into())
        );
        assert_eq!(press_enter_suffix("Press enter"), Some(String::new()));
        assert_eq!(press_enter_suffix("Did you press enter? Yes."), None);
        assert_eq!(press_enter_suffix("Don't express enter."), None);
        assert_eq!(press_enter_suffix("ok"), None);
    }

    #[test]
    fn press_enter_is_pressed_asked_about_or_left_as_words() {
        let ship = "Ship it, press enter.";
        assert_eq!(
            press_enter(ship, true, false),
            (PressEnter::Press, Some("Ship it".into()))
        );
        assert_eq!(
            press_enter(ship, false, true),
            (PressEnter::Ask, Some("Ship it".into()))
        );
        assert_eq!(press_enter(ship, false, false), (PressEnter::No, None));
        assert_eq!(press_enter("Ship it.", true, true), (PressEnter::No, None));
    }

    #[test]
    fn line_and_paragraph_commands_insert_breaks() {
        assert_eq!(
            line_breaks("Hi Sam, new line. How are you?"),
            "Hi Sam,\nHow are you?"
        );
        assert_eq!(
            line_breaks("First point new paragraph second point"),
            "First point\n\nSecond point"
        );
        assert_eq!(
            line_breaks("one next line two line break three"),
            "one\nTwo\nThree"
        );
        assert_eq!(line_breaks("Start a new paragraph. Hello"), "\n\nHello");
        assert_eq!(
            line_breaks("Thanks for coming. Start a new paragraph. The slides follow."),
            "Thanks for coming.\n\nThe slides follow."
        );
        assert_eq!(line_breaks("Hi skip a line bye"), "Hi\nBye");
    }

    #[test]
    fn content_uses_are_left_alone() {
        let text = "We launched a new line of shoes and read the next line";
        assert_eq!(line_breaks(text), text);
        assert_eq!(
            line_breaks("launch new line of shoes"),
            "launch new line of shoes"
        );
    }

    #[test]
    fn punctuation_words_become_symbols() {
        assert_eq!(
            punctuation("Hello comma how are you question mark"),
            "Hello, how are you?"
        );
        assert_eq!(
            punctuation("Hello, comma, how are you"),
            "Hello, how are you"
        );
        assert_eq!(
            punctuation("Done period See you exclamation point"),
            "Done. See you!"
        );
        assert_eq!(punctuation("wait em dash really"), "wait—really");
        assert_eq!(punctuation("wait em-dash really"), "wait—really");
        assert_eq!(punctuation("wait emdash really"), "wait—really");
        assert_eq!(punctuation("Note colon Bring snacks"), "Note: Bring snacks");
    }

    #[test]
    fn punctuation_words_used_as_content_stay() {
        let text = "The trial period ended, so remove the comma and the period";
        assert_eq!(punctuation(text), text);
    }
}
