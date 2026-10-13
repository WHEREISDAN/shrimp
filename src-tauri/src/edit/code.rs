//! Dictation in developer apps (DEV-02, DEV-03): spoken identifiers become
//! the identifiers on screen ("user ID" → `userId`), spoken case commands
//! build identifiers ("camel case user profile" → `userProfile`), common
//! acronyms and tool names get their usual spelling, and shell commands
//! dictated into a terminal are written as commands.

use crate::context::{DictationContext, terms};

/// The developer-app steps of the edit pipeline, with the identifiers on screen.
pub struct DevPass {
    identifiers: Vec<String>,
}

impl DevPass {
    /// Only for code editors and terminals (DEV-03).
    pub fn new(context: &DictationContext) -> Option<Self> {
        context.is_developer().then(|| Self {
            identifiers: context.identifiers(),
        })
    }

    /// Case commands, identifiers on screen, then acronyms and tool names.
    /// A dictation that is only an identifier loses its final period.
    pub fn apply(&self, text: &str) -> String {
        let text = apply_case_commands(text);
        let text = apply_dev_terms(&apply_identifiers(&text, &self.identifiers));
        let bare = text.trim().trim_end_matches(['.', '!', '?']);
        let identifier = !bare.is_empty()
            && bare
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '$'));
        if identifier && bare != text.trim() && !terms::identifiers(bare).is_empty() {
            bare.to_owned()
        } else {
            text
        }
    }

    /// Identifiers in `text`, which the AI edit must keep word for word.
    pub fn identifiers_in(text: &str) -> Vec<String> {
        terms::identifiers(text)
    }
}

/// Words that end a spoken case command's identifier.
const CASE_STOPS: [&str; 16] = [
    "to", "equals", "equal", "is", "are", "and", "or", "with", "for", "from", "in", "on", "at",
    "by", "as", "then",
];
const MAX_CASE_WORDS: usize = 5;

/// Spellings developers expect, by their lowercase form.
const DEV_TERMS: &[(&str, &str)] = &[
    ("api", "API"),
    ("apis", "APIs"),
    ("json", "JSON"),
    ("http", "HTTP"),
    ("https", "HTTPS"),
    ("url", "URL"),
    ("urls", "URLs"),
    ("cli", "CLI"),
    ("sql", "SQL"),
    ("html", "HTML"),
    ("css", "CSS"),
    ("ui", "UI"),
    ("ux", "UX"),
    ("cpu", "CPU"),
    ("gpu", "GPU"),
    ("sdk", "SDK"),
    ("aws", "AWS"),
    ("jwt", "JWT"),
    ("oauth", "OAuth"),
    ("uuid", "UUID"),
    ("yaml", "YAML"),
    ("toml", "TOML"),
    ("xml", "XML"),
    ("id", "ID"),
    ("ids", "IDs"),
    ("npm", "npm"),
    ("pnpm", "pnpm"),
    ("github", "GitHub"),
    ("gitlab", "GitLab"),
    ("javascript", "JavaScript"),
    ("typescript", "TypeScript"),
    ("nodejs", "Node.js"),
    ("postgresql", "PostgreSQL"),
    ("mysql", "MySQL"),
    ("sqlite", "SQLite"),
    ("graphql", "GraphQL"),
    ("macos", "macOS"),
    ("ios", "iOS"),
    ("xcode", "Xcode"),
    ("vscode", "VS Code"),
    ("kubernetes", "Kubernetes"),
    ("webpack", "webpack"),
    ("localhost", "localhost"),
];

/// Programs a dictated shell command starts with.
const COMMANDS: &[&str] = &[
    "git",
    "npm",
    "npx",
    "pnpm",
    "yarn",
    "bun",
    "cargo",
    "rustup",
    "docker",
    "kubectl",
    "brew",
    "ls",
    "cd",
    "cat",
    "mkdir",
    "rm",
    "cp",
    "mv",
    "touch",
    "grep",
    "rg",
    "find",
    "curl",
    "wget",
    "ssh",
    "scp",
    "python",
    "python3",
    "pip",
    "pip3",
    "node",
    "deno",
    "make",
    "go",
    "rustc",
    "sudo",
    "chmod",
    "chown",
    "echo",
    "export",
    "source",
    "open",
    "code",
    "cursor",
    "claude",
    "codex",
    "gh",
    "tar",
    "unzip",
    "kill",
    "pkill",
    "ps",
    "top",
    "htop",
    "vim",
    "nano",
    "less",
    "head",
    "tail",
    "which",
    "man",
    "ping",
    "uv",
    "poetry",
    "terraform",
    "aws",
    "gcloud",
];

/// Git subcommands, which also tell "git" misheard as "get".
const GIT_SUBCOMMANDS: &[&str] = &[
    "add",
    "branch",
    "checkout",
    "cherry-pick",
    "clone",
    "commit",
    "diff",
    "fetch",
    "init",
    "log",
    "merge",
    "pull",
    "push",
    "rebase",
    "remote",
    "reset",
    "restore",
    "show",
    "stash",
    "status",
    "switch",
    "tag",
];

/// A text piece: a word (letters, digits, apostrophes) or what lies between words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Piece<'a> {
    text: &'a str,
    word: bool,
}

fn pieces(text: &str) -> Vec<Piece<'_>> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut in_word = false;
    for (index, c) in text.char_indices() {
        let is_word = c.is_alphanumeric() || c == '\'' || c == '’';
        if index > start && is_word != in_word {
            pieces.push(Piece {
                text: &text[start..index],
                word: in_word,
            });
            start = index;
        }
        in_word = is_word;
    }
    if start < text.len() {
        pieces.push(Piece {
            text: &text[start..],
            word: in_word,
        });
    }
    pieces
}

/// The lowercase words of an identifier: `fetchUserProfile` → fetch, user,
/// profile; `MAX_RETRY` → max, retry; `HTTPServer` → http, server.
pub fn identifier_words(identifier: &str) -> Vec<String> {
    let mut words = Vec::new();
    for part in identifier.split(|c: char| !c.is_ascii_alphanumeric()) {
        let chars: Vec<char> = part.chars().collect();
        let mut current = String::new();
        for (index, &c) in chars.iter().enumerate() {
            let previous = index.checked_sub(1).map(|i| chars[i]);
            let next = chars.get(index + 1);
            let boundary = c.is_ascii_uppercase()
                && previous.is_some_and(|p| {
                    p.is_ascii_lowercase()
                        || p.is_ascii_digit()
                        || (p.is_ascii_uppercase() && next.is_some_and(char::is_ascii_lowercase))
                });
            if boundary && !current.is_empty() {
                words.push(std::mem::take(&mut current).to_ascii_lowercase());
            }
            current.push(c);
        }
        if !current.is_empty() {
            words.push(current.to_ascii_lowercase());
        }
    }
    words
}

fn spoken_key(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// Replaces spoken forms of `identifiers` with the identifiers (DEV-02).
/// Only identifiers of two or more words are matched, longest first.
pub fn apply_identifiers(text: &str, identifiers: &[String]) -> String {
    let collapsed = collapse_dotted_letters(text);
    let text = collapsed.as_str();
    let mut candidates: Vec<(&str, Vec<String>)> = identifiers
        .iter()
        .map(|identifier| (identifier.as_str(), identifier_words(identifier)))
        .filter(|(_, words)| words.len() >= 2)
        .collect();
    candidates.sort_by_key(|(_, words)| std::cmp::Reverse(words.len()));
    if candidates.is_empty() {
        return text.to_owned();
    }
    let pieces = pieces(text);
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    'outer: while index < pieces.len() {
        if pieces[index].word {
            for (identifier, words) in &candidates {
                if let Some(end) = match_words(&pieces, index, words) {
                    out.push_str(identifier);
                    index = end;
                    continue 'outer;
                }
            }
        }
        out.push_str(pieces[index].text);
        index += 1;
    }
    out
}

/// "I.D." → "ID": capitals spelled with dots become one word ("e.g." stays).
fn collapse_dotted_letters(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        let starts_word = index == 0 || !chars[index - 1].is_alphanumeric();
        let mut end = index;
        let mut letters = String::new();
        while starts_word
            && end + 1 < chars.len()
            && chars[end].is_uppercase()
            && chars[end + 1] == '.'
            && chars
                .get(end + 2)
                .is_none_or(|c| !c.is_alphanumeric() || chars.get(end + 3) == Some(&'.'))
        {
            letters.push(chars[end]);
            end += 2;
        }
        if letters.chars().count() >= 2 {
            out.push_str(&letters);
            index = end;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }
    out
}

/// The piece index after `words` matched from `start`, joined by spaces or
/// hyphens only.
fn match_words(pieces: &[Piece<'_>], start: usize, words: &[String]) -> Option<usize> {
    let mut index = start;
    for (n, word) in words.iter().enumerate() {
        if n > 0 {
            let gap = pieces.get(index)?;
            if gap.word || !gap.text.chars().all(|c| c == ' ' || c == '-' || c == '.') {
                return None;
            }
            index += 1;
        }
        let piece = pieces.get(index)?;
        if !piece.word || spoken_key(piece.text) != *word {
            return None;
        }
        index += 1;
    }
    // "I.D." ends in a dot that belonged to the identifier.
    Some(index)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Case {
    Camel,
    Pascal,
    Snake,
    Kebab,
    Constant,
}

fn case_named(first: &str, second: Option<&str>) -> Option<(Case, usize)> {
    let first = first.to_lowercase();
    let second = second.map(str::to_lowercase);
    let case = match (first.as_str(), second.as_deref()) {
        ("camel", Some("case")) => (Case::Camel, 2),
        ("pascal", Some("case")) => (Case::Pascal, 2),
        ("snake", Some("case")) => (Case::Snake, 2),
        ("kebab", Some("case")) => (Case::Kebab, 2),
        ("constant", Some("case")) => (Case::Constant, 2),
        ("camelcase", _) => (Case::Camel, 1),
        ("pascalcase", _) => (Case::Pascal, 1),
        ("snakecase", _) => (Case::Snake, 1),
        _ => return None,
    };
    Some(case)
}

fn join_case(case: Case, words: &[String]) -> String {
    let capitalized = |word: &String| {
        let mut chars = word.chars();
        chars
            .next()
            .map(|c| c.to_uppercase().chain(chars).collect::<String>())
            .unwrap_or_default()
    };
    match case {
        Case::Camel => words
            .iter()
            .enumerate()
            .map(|(index, word)| {
                if index == 0 {
                    word.clone()
                } else {
                    capitalized(word)
                }
            })
            .collect(),
        Case::Pascal => words.iter().map(capitalized).collect(),
        Case::Snake => words.join("_"),
        Case::Kebab => words.join("-"),
        Case::Constant => words.join("_").to_uppercase(),
    }
}

/// "camel case user profile" → `userProfile`, and the same for pascal,
/// snake, kebab and constant case (DEV-03).
pub fn apply_case_commands(text: &str) -> String {
    let pieces = pieces(text);
    let words: Vec<usize> = (0..pieces.len()).filter(|&i| pieces[i].word).collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < pieces.len() {
        let position = words.iter().position(|&w| w == index);
        let command = position.and_then(|p| {
            let second = words.get(p + 1).map(|&w| pieces[w].text);
            case_named(pieces[index].text, second).map(|(case, used)| (case, p + used))
        });
        let Some((case, next_word)) = command else {
            out.push_str(pieces[index].text);
            index += 1;
            continue;
        };
        let mut taken: Vec<String> = Vec::new();
        let mut last_piece = None;
        for &w in words.iter().skip(next_word).take(MAX_CASE_WORDS) {
            let gap_ok = last_piece
                .is_none_or(|last: usize| w == last + 2 && pieces[last + 1].text.trim().is_empty());
            let word = pieces[w].text.to_lowercase();
            if !gap_ok || CASE_STOPS.contains(&word.as_str()) {
                break;
            }
            // The first word may follow "case," with a comma.
            if last_piece.is_none() {
                let between = &pieces[words[next_word - 1] + 1..w];
                if between
                    .iter()
                    .any(|piece| piece.word || !piece.text.trim_matches([' ', ',']).is_empty())
                {
                    break;
                }
            }
            // Whisper may already have joined them: "CamelCase UserProfile".
            taken.extend(identifier_words(pieces[w].text));
            last_piece = Some(w);
        }
        let Some(last) = last_piece else {
            out.push_str(pieces[index].text);
            index += 1;
            continue;
        };
        out.push_str(&join_case(case, &taken));
        index = last + 1;
    }
    out
}

/// Common acronyms and tool names in their usual spelling (DEV-03).
pub fn apply_dev_terms(text: &str) -> String {
    let pieces = pieces(text);
    let mut out = String::with_capacity(text.len());
    // A separator without spaces between two words joins them into a path,
    // a file name or an identifier, which keep their spelling.
    let joins = |separator: usize, beyond: Option<usize>| {
        let separator = &pieces[separator];
        !separator.word
            && !separator.text.chars().any(char::is_whitespace)
            && beyond
                .and_then(|i| pieces.get(i))
                .is_some_and(|piece| piece.word)
    };
    for (index, piece) in pieces.iter().enumerate() {
        let before = index
            .checked_sub(1)
            .is_some_and(|separator| joins(separator, separator.checked_sub(1)));
        let after = index + 1 < pieces.len() && joins(index + 1, Some(index + 2));
        let in_code = before || after;
        let replacement = (piece.word && !in_code)
            .then(|| {
                DEV_TERMS
                    .iter()
                    .find(|(key, _)| piece.text.eq_ignore_ascii_case(key))
            })
            .flatten()
            .map(|(_, term)| *term);
        out.push_str(replacement.unwrap_or(piece.text));
    }
    out
}

/// The words of a dictated command, repaired: "get" before a git subcommand
/// is "git", and "checkout-b" is "checkout -b".
fn command_words(text: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        let word = word.trim_end_matches([',', '.', '?', '!']);
        let split = word.rsplit_once('-').filter(|(prefix, suffix)| {
            let prefix = prefix.to_lowercase();
            (COMMANDS.contains(&prefix.as_str()) || GIT_SUBCOMMANDS.contains(&prefix.as_str()))
                && (1..=3).contains(&suffix.len())
                && suffix.chars().all(|c| c.is_ascii_alphabetic())
        });
        match split {
            Some((prefix, suffix)) => {
                words.push(prefix.to_owned());
                words.push(format!("-{suffix}"));
            }
            None if !word.is_empty() => words.push(word.to_owned()),
            None => {}
        }
    }
    let misheard_git = words.first().is_some_and(|w| w.eq_ignore_ascii_case("get"))
        && words
            .get(1)
            .is_some_and(|w| GIT_SUBCOMMANDS.contains(&w.to_lowercase().as_str()));
    if misheard_git {
        words[0] = "git".into();
    }
    words
}

/// A dictation that starts with a shell program is a command (DEV-03).
pub fn looks_like_command(text: &str) -> bool {
    command_words(text)
        .first()
        .map(|word| {
            word.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .is_some_and(|word| COMMANDS.contains(&word.as_str()))
}

/// Writes a dictated shell command: spoken symbols become symbols ("dash
/// dash release" → `--release`, "feature slash login" → `feature/login`),
/// and there is no capital or final period.
pub fn format_command(text: &str) -> String {
    let words = command_words(text);
    let mut out = String::new();
    let mut glue = false;
    // The last word belongs to a flag, so a dash continues it: --save-dev.
    let mut in_flag = false;
    let mut index = 0;
    while index < words.len() {
        let word = &words[index];
        if word.starts_with('-') && word.len() > 1 {
            // A flag Whisper already wrote, like the "-b" of "checkout-b".
            if !out.is_empty() && !glue {
                out.push(' ');
            }
            out.push_str(word);
            in_flag = true;
            glue = false;
            index += 1;
            continue;
        }
        let lower = word.to_lowercase();
        let next = words.get(index + 1).map(|w| w.to_lowercase());
        let (symbol, used, joins_next) = match (lower.as_str(), next.as_deref()) {
            ("dash", Some("dash")) => ("--", 2, true),
            ("dash" | "minus", _) => ("-", 1, true),
            ("dot", _) => (".", 1, true),
            ("slash", _) => ("/", 1, true),
            ("tilde", _) => ("~", 1, true),
            ("underscore", _) => ("_", 1, true),
            ("equals", _) => ("=", 1, true),
            ("pipe", _) => ("|", 1, false),
            ("star" | "asterisk", _) => ("*", 1, false),
            _ => ("", 0, false),
        };
        let joins_previous = matches!(symbol, "." | "/" | "_" | "=") || (symbol == "-" && in_flag);
        if used > 0 {
            if !out.is_empty() && !glue && !joins_previous {
                out.push(' ');
            }
            out.push_str(symbol);
            in_flag = symbol.starts_with('-') || (in_flag && symbol == "=");
            glue = joins_next;
            index += used;
            continue;
        }
        if !out.is_empty() && !glue {
            out.push(' ');
            in_flag = false;
        }
        if out.is_empty() {
            out.push_str(&lower);
        } else {
            out.push_str(word);
        }
        glue = false;
        index += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|id| (*id).to_owned()).collect()
    }

    #[test]
    fn splits_identifiers_into_words() {
        assert_eq!(
            identifier_words("fetchUserProfile"),
            ["fetch", "user", "profile"]
        );
        assert_eq!(
            identifier_words("MAX_RETRY_COUNT"),
            ["max", "retry", "count"]
        );
        assert_eq!(identifier_words("HTTPServer"), ["http", "server"]);
        assert_eq!(identifier_words("userId"), ["user", "id"]);
    }

    #[test]
    fn spoken_identifiers_become_the_ones_on_screen() {
        let screen = ids(&["userId", "maxRetryCount", "fetchUserProfile"]);
        assert_eq!(
            apply_identifiers("Set the user ID to the max retry count.", &screen),
            "Set the userId to the maxRetryCount."
        );
        assert_eq!(
            apply_identifiers("Call fetch user profile, then user I.D. again", &screen),
            "Call fetchUserProfile, then userId again"
        );
        assert_eq!(
            apply_identifiers("e.g. the U.R.L. is fine", &screen),
            "e.g. the URL is fine"
        );
        assert_eq!(
            apply_identifiers("the user said hi", &screen),
            "the user said hi",
            "a partial match stays"
        );
    }

    #[test]
    fn case_commands_build_identifiers() {
        assert_eq!(
            apply_case_commands("camel case user profile"),
            "userProfile"
        );
        assert_eq!(
            apply_case_commands("Rename it to snake case retry count and save."),
            "Rename it to retry_count and save."
        );
        assert_eq!(apply_case_commands("pascal case, user store"), "UserStore");
        assert_eq!(apply_case_commands("constant case max size"), "MAX_SIZE");
        assert_eq!(apply_case_commands("kebab case main menu"), "main-menu");
        assert_eq!(
            apply_case_commands("set camel case is ready to true"),
            "set camel case is ready to true",
            "a stop word right away means no identifier"
        );
        assert_eq!(
            apply_case_commands("a snake in the grass"),
            "a snake in the grass"
        );
    }

    #[test]
    fn a_lone_identifier_loses_its_final_period() {
        let pass = DevPass {
            identifiers: Vec::new(),
        };
        assert_eq!(pass.apply("Camel case user profile."), "userProfile");
        assert_eq!(pass.apply("Done."), "Done.");
    }

    #[test]
    fn acronyms_and_tools_get_their_usual_spelling() {
        assert_eq!(
            apply_dev_terms("Call the api and parse the json from github."),
            "Call the API and parse the JSON from GitHub."
        );
        assert_eq!(
            apply_dev_terms("open src/api/json.ts"),
            "open src/api/json.ts"
        );
        assert_eq!(apply_dev_terms("the user id"), "the user ID");
    }

    #[test]
    fn shell_commands_are_written_as_commands() {
        assert!(looks_like_command("Git status."));
        assert!(!looks_like_command("Get the status."));
        assert_eq!(format_command("Git status."), "git status");
        assert_eq!(
            format_command("git checkout dash b feature slash login"),
            "git checkout -b feature/login"
        );
        assert_eq!(
            format_command("Cargo test dash dash release."),
            "cargo test --release"
        );
        assert_eq!(
            format_command("ls dash la tilde slash projects"),
            "ls -la ~/projects"
        );
        assert_eq!(
            format_command("npm install dash dash save dash dev typescript"),
            "npm install --save-dev typescript"
        );
        assert_eq!(format_command("cat notes dot txt"), "cat notes.txt");
        assert!(looks_like_command("get checkout-b feature slash login."));
        assert_eq!(
            format_command("get checkout-b feature slash login."),
            "git checkout -b feature/login"
        );
        assert!(!looks_like_command("Get the milk."));
    }
}
