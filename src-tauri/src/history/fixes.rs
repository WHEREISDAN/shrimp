//! How many words the AI edit corrected (STA-01): the dictated words that
//! did not survive into the inserted text, found with a word-level longest
//! common subsequence that ignores case and punctuation.

/// Longer dictations are compared in their first this-many words, which
/// keeps the table small.
const MAX_WORDS: usize = 2_000;

pub fn words_corrected(raw: &str, edited: &str) -> i64 {
    removed_words(raw, edited).len() as i64
}

/// The dictated words (lowercase, without punctuation) the edit removed or
/// changed, in order.
pub fn removed_words(raw: &str, edited: &str) -> Vec<String> {
    let raw = words(raw);
    let edited = words(edited);
    if raw.is_empty() {
        return Vec::new();
    }
    let table = lengths(&raw, &edited);
    let (mut i, mut j) = (raw.len(), edited.len());
    let mut removed = Vec::new();
    while i > 0 {
        if j > 0 && raw[i - 1] == edited[j - 1] {
            i -= 1;
            j -= 1;
        } else if j > 0 && table[i][j - 1] >= table[i - 1][j] {
            j -= 1;
        } else {
            removed.push(raw[i - 1].clone());
            i -= 1;
        }
    }
    removed.reverse();
    removed
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric() || *c == '\'')
                .flat_map(char::to_lowercase)
                .collect::<String>()
        })
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .take(MAX_WORDS)
        .collect()
}

/// `table[i][j]`: the longest common subsequence of `a[..i]` and `b[..j]`.
fn lengths(a: &[String], b: &[String]) -> Vec<Vec<u32>> {
    let mut table = vec![vec![0u32; b.len() + 1]; a.len() + 1];
    for (i, word) in a.iter().enumerate() {
        for (j, other) in b.iter().enumerate() {
            table[i + 1][j + 1] = if word == other {
                table[i][j] + 1
            } else {
                table[i][j + 1].max(table[i + 1][j])
            };
        }
    }
    table
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
        assert_eq!(
            removed_words(
                "um so let's do coffee at two actually three",
                "Let's do coffee at 3."
            ),
            ["um", "so", "two", "actually", "three"]
        );
    }
}
