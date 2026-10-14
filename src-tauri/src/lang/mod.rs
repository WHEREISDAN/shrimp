//! Languages (LANG-01..05, LANG-07): the catalog, what the speech engine is
//! told, how a variant shapes the output, and the region's variant.

pub mod catalog;
pub mod french;
pub mod region;
pub mod spelling;

use crate::settings::LanguageSettings;
use spelling::Spelling;

/// The Whisper languages to transcribe in: none for auto-detect, the one
/// chosen from the Flow Bar, or the user's languages (LANG-02).
pub fn stt_languages(settings: &LanguageSettings) -> Vec<String> {
    if settings.auto_detect {
        return Vec::new();
    }
    let ids: Vec<&String> = match &settings.active {
        Some(active) => vec![active],
        None => settings.selected.iter().collect(),
    };
    let mut codes: Vec<String> = Vec::new();
    for id in ids {
        let code = catalog::whisper_code(id).to_owned();
        if !codes.contains(&code) {
            codes.push(code);
        }
    }
    codes
}

/// The user's variant for a transcript in `detected` (a Whisper code).
pub fn variant<'a>(settings: &'a LanguageSettings, detected: Option<&str>) -> Option<&'a str> {
    if let Some(active) = &settings.active {
        return Some(active);
    }
    match detected {
        Some(code) => settings
            .selected
            .iter()
            .find(|id| catalog::whisper_code(id) == code)
            .map(String::as_str),
        None => match settings.selected.as_slice() {
            [only] => Some(only),
            _ => None,
        },
    }
}

/// What the AI edit is told about the variant (LANG-03).
pub fn prompt_rule(id: &str) -> Option<&'static str> {
    Some(match id {
        "en-US" => "Use American English spelling (color, organize, traveled).",
        "en-GB" => "Use British English spelling (colour, organise, travelled).",
        "en-CA" => "Use Canadian English spelling (colour, organize, travelled).",
        "zh-Hans" => "Write Chinese in Simplified characters.",
        "zh-Hant" => "Write Chinese in Traditional characters.",
        "yue" => "Write Cantonese in Traditional Chinese characters.",
        "de-CH" => "Use Swiss Standard German spelling: write ss, never ß.",
        _ => return None,
    })
}

/// Text that leads Whisper's prompt towards the variant's script. Whisper
/// often writes Simplified anyway, so [`shape`] converts the characters.
pub fn whisper_hint(id: &str) -> Option<&'static str> {
    Some(match id {
        "zh-Hans" => "以下是普通话的句子。",
        "zh-Hant" => "以下是普通話的句子。",
        "yue" => "以下係廣東話嘅句子。",
        _ => return None,
    })
}

/// Hints for every selected language, by Whisper code.
pub fn whisper_hints(settings: &LanguageSettings) -> Vec<(String, String)> {
    settings
        .selected
        .iter()
        .filter_map(|id| {
            whisper_hint(id).map(|hint| (catalog::whisper_code(id).to_owned(), hint.to_owned()))
        })
        .collect()
}

/// Whether French typography applies (FMT-14): the dictation was in French.
pub fn is_french(settings: &LanguageSettings, detected: Option<&str>) -> bool {
    match detected {
        Some(code) => code == "fr",
        None => variant(settings, None).is_some_and(|id| catalog::whisper_code(id) == "fr"),
    }
}

/// Deterministic spelling for the variant, after any AI edit.
pub fn shape(text: &str, id: Option<&str>) -> String {
    match id.and_then(Spelling::for_language) {
        Some(spelling) => spelling::apply(text, spelling),
        None => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(auto_detect: bool, selected: &[&str], active: Option<&str>) -> LanguageSettings {
        LanguageSettings {
            auto_detect,
            selected: selected.iter().map(|id| (*id).to_owned()).collect(),
            active: active.map(str::to_owned),
        }
    }

    #[test]
    fn the_engine_hears_codes_auto_detect_or_the_active_language() {
        assert!(stt_languages(&settings(true, &["fr"], None)).is_empty());
        assert_eq!(
            stt_languages(&settings(false, &["en-GB", "fr", "de-CH"], None)),
            ["en", "fr", "de"]
        );
        assert_eq!(
            stt_languages(&settings(false, &["en-GB", "fr"], Some("fr"))),
            ["fr"]
        );
    }

    #[test]
    fn the_variant_follows_the_detected_language() {
        let mine = settings(false, &["en-GB", "zh-Hant"], None);
        assert_eq!(variant(&mine, Some("en")), Some("en-GB"));
        assert_eq!(variant(&mine, Some("zh")), Some("zh-Hant"));
        assert_eq!(variant(&mine, Some("fr")), None);
        assert_eq!(
            variant(&settings(false, &["en-CA"], None), None),
            Some("en-CA")
        );
        assert_eq!(
            shape("My favorite color", variant(&mine, Some("en"))),
            "My favourite colour"
        );
        assert_eq!(
            whisper_hints(&mine),
            [("zh".to_owned(), "以下是普通話的句子。".to_owned())]
        );
    }
}
