//! Spells names the way the screen shows them (CTX-02): a capitalized word
//! in the transcript that sounds like a name near the cursor or on screen
//! ("Catherine" when "Kathryn" is visible) takes the visible spelling.

use crate::context::words;

/// Rewrites capitalized words that sound like one of `names`. `known` words
/// (dictionary terms) are left alone.
pub fn respell(text: &str, names: &[String], known: &[String]) -> String {
    let candidates: Vec<(&str, String)> = names
        .iter()
        .filter(|name| name.chars().count() >= 3 && name.chars().all(char::is_alphabetic))
        .map(|name| (name.as_str(), sound_key(name)))
        .collect();
    if candidates.is_empty() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(char::is_alphabetic) {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let end = rest
            .find(|c: char| !c.is_alphabetic())
            .unwrap_or(rest.len());
        let word = &rest[..end];
        out.push_str(replacement(word, &candidates, known).unwrap_or(word));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn replacement<'a>(word: &str, names: &[(&'a str, String)], known: &[String]) -> Option<&'a str> {
    let looks_like_name = word.chars().count() >= 3
        && word.starts_with(char::is_uppercase)
        && word.chars().skip(1).all(char::is_lowercase)
        && !words::is_common(word);
    if !looks_like_name || known.iter().any(|term| term == word) {
        return None;
    }
    let key = sound_key(word);
    let lower = word.to_lowercase();
    names.iter().find_map(|(name, name_key)| {
        let name_lower = name.to_lowercase();
        let close = distance(&lower, &name_lower) <= lower.len().max(name_lower.len()) / 2;
        (*name != word && *name_key == key && close).then_some(*name)
    })
}

/// A rough phonetic key: consonant sounds after the first letter, with
/// common spelling variants folded together (c/k, ph/f, th/t, y/i).
fn sound_key(word: &str) -> String {
    let lower: String = word
        .to_lowercase()
        .chars()
        .filter(char::is_ascii_alphabetic)
        .collect();
    let mut folded = lower
        .replace("ph", "f")
        .replace("ck", "k")
        .replace("sch", "sk")
        .replace("sh", "s")
        .replace("th", "t")
        .replace("qu", "k")
        .replace('x', "ks")
        .replace('z', "s")
        .replace('v', "f")
        .replace('y', "i");
    folded = soften_c(&folded).replace('q', "k");
    let mut key = String::new();
    for (index, c) in folded.chars().enumerate() {
        let silent = index > 0 && matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'h' | 'w');
        if !silent && !key.ends_with(c) {
            key.push(c);
        }
    }
    key
}

/// "c" sounds like "s" before e, i and y, and like "k" otherwise.
fn soften_c(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    chars
        .iter()
        .enumerate()
        .map(|(index, &c)| match c {
            'c' if matches!(chars.get(index + 1), Some('e' | 'i' | 'y')) => 's',
            'c' => 'k',
            other => other,
        })
        .collect()
}

/// Levenshtein distance between two lowercase ASCII-ish words.
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let substitute = previous[j] + usize::from(ca != cb);
            current.push(substitute.min(previous[j + 1] + 1).min(current[j] + 1));
        }
        previous = current;
    }
    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn takes_the_spelling_on_screen() {
        assert_eq!(
            respell(
                "Please forward this to Catherine and Sean tomorrow.",
                &names(&["Kathryn", "Shawn"]),
                &[]
            ),
            "Please forward this to Kathryn and Shawn tomorrow."
        );
        assert_eq!(
            respell("Ask Steven about it.", &names(&["Stephen"]), &[]),
            "Ask Stephen about it."
        );
        assert_eq!(
            respell("Catherine's notes", &names(&["Kathryn"]), &[]),
            "Kathryn's notes"
        );
    }

    #[test]
    fn leaves_everyday_words_and_other_names_alone() {
        let screen = names(&["Kathryn", "Nguyen"]);
        assert_eq!(
            respell("The cat ran when Mark called.", &screen, &[]),
            "The cat ran when Mark called."
        );
        assert_eq!(
            respell("Catherine is here.", &screen, &names(&["Catherine"])),
            "Catherine is here.",
            "a dictionary term wins"
        );
        assert_eq!(respell("Catherine", &[], &[]), "Catherine");
    }

    #[test]
    fn sound_keys_fold_spelling_variants() {
        assert_eq!(sound_key("Catherine"), sound_key("Kathryn"));
        assert_eq!(sound_key("Philip"), sound_key("Filip"));
        assert_ne!(sound_key("Nguyen"), sound_key("when"));
    }
}
