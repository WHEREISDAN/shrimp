//! The variant the system region speaks: English (Canada) for people in
//! Canada who dictate in another English variant (LANG-07).

use super::catalog;
use crate::settings::LanguageSettings;

pub const CANADIAN_ENGLISH: &str = "en-CA";
const CANADA: &str = "CA";

/// Whether to suggest English (Canada): the region is Canada and the user
/// dictates in another English variant.
pub fn suggests_canadian_english(region: Option<&str>, languages: &LanguageSettings) -> bool {
    region.is_some_and(|region| region.eq_ignore_ascii_case(CANADA))
        && with_canadian_english(languages).is_some()
}

/// The languages with English (Canada) in place of the other English
/// variant, in its place in the list and as the Flow Bar's choice if it was
/// that. None when no other English variant is selected, or Canadian is.
pub fn with_canadian_english(languages: &LanguageSettings) -> Option<LanguageSettings> {
    let english = catalog::group(CANADIAN_ENGLISH)?;
    if languages.selected.iter().any(|id| id == CANADIAN_ENGLISH) {
        return None;
    }
    let index = languages
        .selected
        .iter()
        .position(|id| catalog::group(id) == Some(english))?;
    let mut switched = languages.clone();
    let replaced = std::mem::replace(&mut switched.selected[index], CANADIAN_ENGLISH.to_owned());
    if switched.active.as_deref() == Some(replaced.as_str()) {
        switched.active = Some(CANADIAN_ENGLISH.to_owned());
    }
    Some(switched)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn languages(selected: &[&str], active: Option<&str>) -> LanguageSettings {
        LanguageSettings {
            auto_detect: false,
            selected: selected.iter().map(|id| (*id).to_owned()).collect(),
            active: active.map(str::to_owned),
        }
    }

    #[test]
    fn only_canada_with_another_english_is_offered_the_switch() {
        let american = languages(&["en-US"], None);
        assert!(suggests_canadian_english(Some("CA"), &american));
        assert!(suggests_canadian_english(Some("ca"), &american));
        assert!(!suggests_canadian_english(Some("US"), &american));
        assert!(!suggests_canadian_english(None, &american));
        let canadian = languages(&["en-CA"], None);
        assert!(!suggests_canadian_english(Some("CA"), &canadian));
        let french = languages(&["fr"], None);
        assert!(!suggests_canadian_english(Some("CA"), &french));
    }

    #[test]
    fn the_switch_replaces_the_other_english_in_place() {
        let switched = with_canadian_english(&languages(&["fr", "en-GB", "de"], Some("en-GB")));
        assert_eq!(
            switched,
            Some(languages(&["fr", "en-CA", "de"], Some("en-CA")))
        );
        let switched = with_canadian_english(&languages(&["en-US", "fr"], Some("fr")));
        assert_eq!(switched, Some(languages(&["en-CA", "fr"], Some("fr"))));
        switched.unwrap().validate().unwrap();
    }
}
