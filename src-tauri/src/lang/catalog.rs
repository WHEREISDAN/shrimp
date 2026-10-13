//! The languages Shrimp can transcribe (LANG-01, LANG-03): Whisper's own
//! list, with English, Chinese and German split into the variants that
//! shape the output. Choosing one of a pair deselects the other.

use serde::Serialize;

/// Mirrors `LanguageOption` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageOption {
    /// Stored in the settings: a Whisper code ("fr") or a variant ("en-GB").
    pub id: String,
    pub name: String,
    /// Variants in one group exclude each other.
    pub group: Option<&'static str>,
}

/// Variants and the Whisper language they transcribe as.
const VARIANTS: [(&str, &str, &str, Option<&str>); 8] = [
    ("en-US", "English (US)", "en", Some("en")),
    ("en-GB", "English (UK)", "en", Some("en")),
    ("en-CA", "English (Canada)", "en", Some("en")),
    ("zh-Hans", "Chinese (Simplified)", "zh", Some("zh")),
    ("zh-Hant", "Chinese (Traditional)", "zh", Some("zh")),
    ("yue", "Cantonese", "yue", None),
    ("de", "German", "de", Some("de")),
    ("de-CH", "Swiss German", "de", Some("de")),
];

/// Whisper codes the variants replace in the list.
const REPLACED: [&str; 4] = ["en", "zh", "yue", "de"];

fn title_case(name: &str) -> String {
    name.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every language, sorted by name.
pub fn languages() -> Vec<LanguageOption> {
    let whisper = (0..=whisper_rs::get_lang_max_id()).filter_map(|id| {
        let code = whisper_rs::get_lang_str(id)?;
        let name = whisper_rs::get_lang_str_full(id)?;
        (!REPLACED.contains(&code)).then(|| LanguageOption {
            id: code.to_owned(),
            name: title_case(name),
            group: None,
        })
    });
    let variants = VARIANTS.iter().map(|(id, name, _, group)| LanguageOption {
        id: (*id).to_owned(),
        name: (*name).to_owned(),
        group: *group,
    });
    let mut all: Vec<LanguageOption> = whisper.chain(variants).collect();
    all.sort_by(|a, b| a.name.cmp(&b.name));
    all
}

/// The Whisper language a stored id transcribes as.
pub fn whisper_code(id: &str) -> &str {
    VARIANTS
        .iter()
        .find(|(variant, ..)| *variant == id)
        .map_or(id, |(_, _, code, _)| code)
}

pub fn group(id: &str) -> Option<&'static str> {
    VARIANTS
        .iter()
        .find(|(variant, ..)| *variant == id)
        .and_then(|(_, _, _, group)| *group)
}

pub fn is_known(id: &str) -> bool {
    VARIANTS.iter().any(|(variant, ..)| *variant == id)
        || (!REPLACED.contains(&id) && whisper_rs::get_lang_id(id).is_some())
}

/// The language id for a macOS locale such as "en-GB", "zh-Hant-TW" or "fr-CA".
pub fn from_locale(tag: &str) -> Option<String> {
    let mut parts = tag.split(['-', '_']);
    let language = parts.next()?.to_lowercase();
    let rest: Vec<String> = parts.map(str::to_owned).collect();
    let has = |part: &str| rest.iter().any(|other| other.eq_ignore_ascii_case(part));
    let id = match language.as_str() {
        "en" if has("GB") || has("AU") || has("NZ") || has("IE") => "en-GB",
        "en" if has("CA") => "en-CA",
        "en" => "en-US",
        "zh" if has("Hant") || has("TW") || has("HK") || has("MO") => "zh-Hant",
        "zh" => "zh-Hans",
        "de" if has("CH") => "de-CH",
        other => return is_known(other).then(|| other.to_owned()),
    };
    Some(id.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_whisper_languages_with_variants_in_their_place() {
        let all = languages();
        let ids: Vec<&str> = all.iter().map(|language| language.id.as_str()).collect();
        for id in [
            "en-US", "en-GB", "en-CA", "zh-Hans", "zh-Hant", "yue", "de-CH", "fr", "ja",
        ] {
            assert!(ids.contains(&id), "{id} missing");
        }
        assert!(!ids.contains(&"en") && !ids.contains(&"zh"));
        let french = all.iter().find(|language| language.id == "fr").unwrap();
        assert_eq!(french.name, "French");
        let haitian = all.iter().find(|language| language.id == "ht").unwrap();
        assert_eq!(haitian.name, "Haitian Creole");
        assert!(all.len() > 95);
    }

    #[test]
    fn variants_transcribe_as_their_whisper_language() {
        assert_eq!(whisper_code("en-GB"), "en");
        assert_eq!(whisper_code("zh-Hant"), "zh");
        assert_eq!(whisper_code("de-CH"), "de");
        assert_eq!(whisper_code("yue"), "yue");
        assert_eq!(whisper_code("fr"), "fr");
        assert_eq!(group("en-CA"), Some("en"));
        assert_eq!(group("yue"), None);
        assert!(is_known("fr") && is_known("en-GB") && !is_known("en") && !is_known("xx"));
    }

    #[test]
    fn locales_map_to_languages() {
        assert_eq!(from_locale("en-US").as_deref(), Some("en-US"));
        assert_eq!(from_locale("en-AU").as_deref(), Some("en-GB"));
        assert_eq!(from_locale("en").as_deref(), Some("en-US"));
        assert_eq!(from_locale("zh-Hant-TW").as_deref(), Some("zh-Hant"));
        assert_eq!(from_locale("zh-Hans-CN").as_deref(), Some("zh-Hans"));
        assert_eq!(from_locale("de-CH").as_deref(), Some("de-CH"));
        assert_eq!(from_locale("fr-CA").as_deref(), Some("fr"));
        assert_eq!(from_locale("xx-YY"), None);
    }
}
