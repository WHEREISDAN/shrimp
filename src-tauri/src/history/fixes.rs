//! How many words the AI edit corrected (STA-01): the dictated words that
//! did not survive into the inserted text, found with a word-level longest
//! common subsequence that ignores case and punctuation.

/// Longer dictations are compared in their first this-many words, which
/// keeps the table small.
const MAX_WORDS: usize = 2_000;

pub fn words_corrected(raw: &str, edited: &str) -> i64 {
    let raw = words(raw);
    let edited = words(edited);
    if raw.is_empty() {
        return 0;
    }
    (raw.len() - common(&raw, &edited)) as i64
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        })
        .filter(|word| !word.is_empty())
        .take(MAX_WORDS)
        .collect()
}

fn common(a: &[String], b: &[String]) -> usize {
    let mut previous = vec![0usize; b.len() + 1];
    let mut current = vec![0usize; b.len() + 1];
    for word in a {
        for (j, other) in b.iter().enumerate() {
            current[j + 1] = if word == other {
                previous[j] + 1
            } else {
                previous[j + 1].max(current[j])
            };
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_removed_and_changed_words() {
        assert_eq!(
            words_corrected(
                "um so let's do coffee at two actually three",
                "Let's do coffee at 3."
            ),
            5
        );
        assert_eq!(words_corrected("Hello there.", "hello there"), 0);
        assert_eq!(words_corrected("", "Anything"), 0);
    }
}
