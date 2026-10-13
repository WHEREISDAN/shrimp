//! Text Rust shows the user (LANG-06): notices, the tray menu, validation
//! messages and names in pickers, in the interface language. English
//! (`locales/en/*.json`) is the source; a key missing elsewhere falls back
//! to it. Placeholders are `{name}`, filled by [`t!`](crate::t); counted
//! text has `.one` and `.other` keys, chosen by [`tn!`](crate::tn).

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{LazyLock, RwLock};

/// The interface languages, as in Settings › General › App language.
pub const LANGUAGES: [&str; 5] = ["en", "de", "es", "it", "pt"];

type Catalog = HashMap<String, String>;

macro_rules! catalog {
    ($language:literal) => {
        (
            $language,
            [
                include_str!(concat!("../../locales/", $language, "/notices.json")),
                include_str!(concat!("../../locales/", $language, "/errors.json")),
                include_str!(concat!("../../locales/", $language, "/names.json")),
            ],
        )
    };
}

static CATALOGS: LazyLock<HashMap<&'static str, Catalog>> = LazyLock::new(|| {
    [
        catalog!("en"),
        catalog!("de"),
        catalog!("es"),
        catalog!("it"),
        catalog!("pt"),
    ]
    .into_iter()
    .map(|(language, parts)| {
        let catalog = parts
            .into_iter()
            .flat_map(|json| {
                serde_json::from_str::<Catalog>(json)
                    .unwrap_or_else(|error| panic!("locales/{language} is invalid: {error}"))
            })
            .collect();
        (language, catalog)
    })
    .collect()
});

static CURRENT: RwLock<&'static str> = RwLock::new("en");

/// The language for the App language setting: "system" follows `preferred`
/// (the Mac's first language, a BCP 47 tag); anything else unknown is English.
pub fn resolve(setting: &str, preferred: Option<&str>) -> &'static str {
    let wanted = if setting == "system" {
        preferred.unwrap_or("en")
    } else {
        setting
    };
    let base = wanted
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .to_lowercase();
    LANGUAGES
        .into_iter()
        .find(|language| *language == base)
        .unwrap_or("en")
}

pub fn set_language(language: &'static str) {
    *CURRENT.write().unwrap_or_else(|err| err.into_inner()) = language;
}

pub fn language() -> &'static str {
    *CURRENT.read().unwrap_or_else(|err| err.into_inner())
}

/// The text for `key` in `language`, with `{name}` placeholders filled.
pub fn text_in(language: &str, key: &str, args: &[(&str, &dyn Display)]) -> String {
    let template = [language, "en"]
        .into_iter()
        .find_map(|language| CATALOGS.get(language)?.get(key))
        .map_or(key, String::as_str);
    args.iter()
        .fold(template.to_owned(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), &value.to_string())
        })
}

/// The text for `key` in the interface language.
pub fn text(key: &str, args: &[(&str, &dyn Display)]) -> String {
    text_in(language(), key, args)
}

/// `key.one` or `key.other` for `count`, with `{count}` filled.
pub fn plural_in(language: &str, key: &str, count: i64, args: &[(&str, &dyn Display)]) -> String {
    // Only 1 is singular; zero reads as plural in all five languages ("0 palavras").
    let key = format!("{key}.{}", if count == 1 { "one" } else { "other" });
    let count = number_in(language, count);
    let mut all: Vec<(&str, &dyn Display)> = vec![("count", &count)];
    all.extend_from_slice(args);
    text_in(language, &key, &all)
}

pub fn plural(key: &str, count: i64, args: &[(&str, &dyn Display)]) -> String {
    plural_in(language(), key, count, args)
}

/// The name of Whisper language `code` in the interface language, as the
/// Settings pages show it; English keeps the catalog's `english` name.
pub fn language_name(code: &str, english: &str) -> String {
    match language() {
        "en" => english.to_owned(),
        language => {
            crate::platform::language_name(code, language).unwrap_or_else(|| english.to_owned())
        }
    }
}

/// 25000 as "25,000" in English and "25.000" in the other languages.
pub fn number_in(language: &str, value: i64) -> String {
    let separator = if language == "en" { ',' } else { '.' };
    let digits = value.unsigned_abs().to_string();
    let grouped: String = digits
        .chars()
        .enumerate()
        .flat_map(|(index, digit)| {
            let group = index > 0 && (digits.len() - index).is_multiple_of(3);
            group.then_some(separator).into_iter().chain([digit])
        })
        .collect();
    if value < 0 {
        format!("-{grouped}")
    } else {
        grouped
    }
}

pub fn number(value: i64) -> String {
    number_in(language(), value)
}

/// `t!("tray.quit")` or `t!("notice.recordingFailed", message = error)`.
#[macro_export]
macro_rules! t {
    ($key:literal) => {
        $crate::i18n::text($key, &[])
    };
    ($key:literal, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::text(
            $key,
            &[$((stringify!($name), &$value as &dyn ::std::fmt::Display)),+],
        )
    };
}

/// `tn!("notice.stopsIn", minutes)` picks `notice.stopsIn.one` or `.other`
/// and fills `{count}`; named arguments follow as in [`t!`].
#[macro_export]
macro_rules! tn {
    ($key:literal, $count:expr $(, $name:ident = $value:expr)* $(,)?) => {
        $crate::i18n::plural(
            $key,
            ::std::convert::TryInto::<i64>::try_into($count).unwrap_or(i64::MAX),
            &[$((stringify!($name), &$value as &dyn ::std::fmt::Display)),*],
        )
    };
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::*;

    fn placeholders(text: &str) -> BTreeSet<&str> {
        text.split('{')
            .skip(1)
            .filter_map(|rest| rest.split_once('}').map(|(name, _)| name))
            .collect()
    }

    #[test]
    fn every_language_has_every_key_with_the_same_placeholders() {
        let english = &CATALOGS["en"];
        let mut problems = Vec::new();
        for language in LANGUAGES {
            let catalog = &CATALOGS[language];
            for (key, text) in english {
                match catalog.get(key) {
                    None => problems.push(format!("{language} lacks {key}")),
                    Some(translated) if translated.trim().is_empty() => {
                        problems.push(format!("{language} {key} is empty"));
                    }
                    Some(translated) if placeholders(text) != placeholders(translated) => {
                        problems.push(format!("{language} {key} changes the placeholders"));
                    }
                    Some(_) => {}
                }
            }
            problems.extend(
                catalog
                    .keys()
                    .filter(|key| !english.contains_key(*key))
                    .map(|key| format!("{language} has an extra {key}")),
            );
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    #[test]
    fn no_key_is_in_two_files() {
        for language in LANGUAGES {
            let total: usize = ["notices", "errors", "names"]
                .iter()
                .map(|part| {
                    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join(format!("locales/{language}/{part}.json"));
                    let json = std::fs::read_to_string(path).unwrap();
                    serde_json::from_str::<Catalog>(&json).unwrap().len()
                })
                .sum();
            assert_eq!(total, CATALOGS[language].len(), "{language} repeats a key");
        }
    }

    /// The keys `t!` and `tn!` name in `source`, with `tn!`'s plural forms.
    fn used_keys(source: &str) -> Vec<String> {
        source
            .match_indices("!(")
            .filter_map(|(at, _)| {
                let name_start = source[..at]
                    .rfind(|c: char| !c.is_alphanumeric() && c != '_')
                    .map_or(0, |index| index + 1);
                let (key, _) = source[at + 2..]
                    .trim_start()
                    .strip_prefix('"')?
                    .split_once('"')?;
                match &source[name_start..at] {
                    "t" => Some(vec![key.to_owned()]),
                    "tn" => Some(vec![format!("{key}.one"), format!("{key}.other")]),
                    _ => None,
                }
            })
            .flatten()
            .collect()
    }

    /// Every `t!("…")` and `tn!("…")` in the source names English text, and
    /// every English text is used.
    #[test]
    fn the_source_and_the_english_text_match() {
        fn visit(dir: &Path, keys: &mut BTreeSet<String>) {
            for entry in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, keys);
                } else if path.extension().is_some_and(|extension| extension == "rs")
                    && !path.ends_with("i18n/mod.rs")
                {
                    keys.extend(used_keys(&std::fs::read_to_string(&path).unwrap()));
                }
            }
        }
        assert_eq!(
            used_keys("t!(\"a.b\", x = 1); tn!(\n  \"c\", 2); format!(\"d\"); print!(\"e\")"),
            ["a.b", "c.one", "c.other"]
        );
        let mut keys = BTreeSet::new();
        visit(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut keys,
        );
        let english = &CATALOGS["en"];
        let missing: Vec<&String> = keys
            .iter()
            .filter(|key| !english.contains_key(*key))
            .collect();
        assert!(missing.is_empty(), "missing from locales/en: {missing:?}");
        let unused: Vec<&String> = english.keys().filter(|key| !keys.contains(*key)).collect();
        assert!(unused.is_empty(), "unused in locales/en: {unused:?}");
    }

    #[test]
    fn resolves_the_setting_and_falls_back_to_english() {
        assert_eq!(resolve("system", Some("de-CH")), "de");
        assert_eq!(resolve("system", Some("pt_BR")), "pt");
        assert_eq!(resolve("system", Some("fr-FR")), "en");
        assert_eq!(resolve("system", None), "en");
        assert_eq!(resolve("it", Some("de-DE")), "it");
        assert_eq!(text_in("de", "no.such.key", &[]), "no.such.key");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn names_languages_like_the_interface() {
        assert_eq!(
            crate::platform::language_name("fr", "de").as_deref(),
            Some("Französisch")
        );
        assert_eq!(
            crate::platform::language_name("es", "it").as_deref(),
            Some("Spagnolo")
        );
    }

    #[test]
    fn groups_thousands_per_language() {
        assert_eq!(number_in("en", 1_234_567), "1,234,567");
        assert_eq!(number_in("de", 25_000), "25.000");
        assert_eq!(number_in("pt", 999), "999");
        assert_eq!(number_in("en", -1_000), "-1,000");
    }
}
