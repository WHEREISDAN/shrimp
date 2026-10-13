//! File tagging in Cursor and Windsurf chat (DEV-01): saying a visible file's
//! name ("index dot ts"), or "tag", "tagged", "at" or "@" and its base name
//! ("tag my python script"), inserts an @-mention (`@my_python_script.py`);
//! the trigger word is replaced.
//! The longest match wins; anything that matches no file stays as it was.

use super::code::identifier_words;
use crate::context::{DictationContext, apps};
use crate::platform::FieldKind;
use crate::settings::Settings;

const TRIGGERS: [&str; 4] = ["tag", "tagged", "at", "@"];

struct FileName<'a> {
    name: &'a str,
    stem: Vec<String>,
    extension: Vec<String>,
}

impl<'a> FileName<'a> {
    fn new(name: &'a str) -> Option<Self> {
        let (stem, extension) = name.rsplit_once('.')?;
        Some(Self {
            name,
            stem: identifier_words(stem),
            extension: identifier_words(extension),
        })
    }

    /// The spoken forms that name this file, as lowercase words.
    fn spoken_forms(&self, triggered: bool) -> Vec<Vec<String>> {
        let dot = std::iter::once("dot".to_owned());
        let full: Vec<String> = self
            .stem
            .iter()
            .cloned()
            .chain(dot)
            .chain(self.extension.iter().cloned())
            .collect();
        // "index.ts" as written: the dot is the gap between the words.
        let written: Vec<String> = self.stem.iter().chain(&self.extension).cloned().collect();
        let mut forms = vec![full, written];
        if triggered && !self.stem.is_empty() {
            forms.push(self.stem.clone());
        }
        forms
    }
}

fn words_with_spans(text: &str) -> Vec<(usize, usize, String)> {
    let mut words = Vec::new();
    let mut start = None;
    for (index, c) in text.char_indices() {
        let in_word = c.is_alphanumeric() || c == '@';
        match (in_word, start) {
            (true, None) => start = Some(index),
            (false, Some(begin)) => {
                words.push((begin, index, text[begin..index].to_lowercase()));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(begin) = start {
        words.push((begin, text.len(), text[begin..].to_lowercase()));
    }
    words
}

/// Words joined only by spaces (or a single dot inside a file name).
fn joined(text: &str, words: &[(usize, usize, String)], from: usize, count: usize) -> bool {
    (from + 1..from + count).all(|i| {
        let gap = &text[words[i - 1].1..words[i].0];
        gap.chars().all(|c| c == ' ') || gap == "." || gap == "_" || gap == "-"
    })
}

/// Tagging applies in Cursor's and Windsurf's chat input, when turned on in
/// Settings › Vibe coding.
pub fn applies(context: &DictationContext, settings: &Settings) -> bool {
    settings.vibe_coding.file_tagging
        && apps::has_file_tagging(context.app.as_ref())
        && context.field_kind() == FieldKind::Text
}

pub fn tag_files(text: &str, files: &[String]) -> String {
    let files: Vec<FileName<'_>> = files
        .iter()
        .filter_map(|name| FileName::new(name))
        .collect();
    if files.is_empty() {
        return text.to_owned();
    }
    let words = words_with_spans(text);
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut index = 0;
    while index < words.len() {
        let triggered = TRIGGERS.contains(&words[index].2.as_str());
        let start = if triggered { index + 1 } else { index };
        let mut best: Option<(usize, &str)> = None;
        for file in &files {
            for form in file.spoken_forms(triggered) {
                let end = start + form.len();
                let matches = end <= words.len()
                    && words[start..end].iter().map(|w| &w.2).eq(form.iter())
                    && joined(text, &words, start, form.len());
                if matches && best.is_none_or(|(len, _)| form.len() > len) {
                    best = Some((form.len(), file.name));
                }
            }
        }
        // A trigger word that names no file is just a word.
        let Some((len, name)) = best.filter(|(len, _)| *len > 0) else {
            index += 1;
            continue;
        };
        let begin = words[index].0;
        let end = words[start + len - 1].1;
        let already = text[..begin].ends_with('@');
        out.push_str(&text[copied..begin]);
        if !already {
            out.push('@');
        }
        out.push_str(name);
        copied = end;
        index = start + len;
    }
    out.push_str(&text[copied..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_spoken_file_name_becomes_a_mention() {
        let open = files(&["index.ts", "my_python_script.py", ".env", "index.test.ts"]);
        assert_eq!(
            tag_files("Check index dot ts for errors.", &open),
            "Check @index.ts for errors."
        );
        assert_eq!(
            tag_files("Check index.ts for errors.", &open),
            "Check @index.ts for errors."
        );
        assert_eq!(
            tag_files("Explain this code, tag my python script", &open),
            "Explain this code, @my_python_script.py"
        );
        assert_eq!(
            tag_files("Load the dot env file", &open),
            "Load the @.env file"
        );
        assert_eq!(
            tag_files("look at index test dot ts", &open),
            "look @index.test.ts",
            "the longest name wins, and the trigger word goes"
        );
    }

    #[test]
    fn unmatched_words_stay() {
        let open = files(&["index.ts"]);
        assert_eq!(
            tag_files("Meet at noon about the index.", &open),
            "Meet at noon about the index."
        );
        assert_eq!(tag_files("tag the release", &open), "tag the release");
        assert_eq!(tag_files("@index.ts is fine", &open), "@index.ts is fine");
        assert_eq!(tag_files("anything", &[]), "anything");
    }
}
