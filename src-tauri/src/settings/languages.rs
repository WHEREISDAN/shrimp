//! Settings › Languages (LANG-01..05).

use serde::{Deserialize, Serialize};

use crate::lang::catalog;
use crate::t;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LanguageSettings {
    /// Transcribe whatever language is spoken.
    pub auto_detect: bool,
    /// The languages the user speaks: Whisper codes and variants ("en-GB").
    /// Empty until first launch fills in the Mac's language.
    pub selected: Vec<String>,
    /// One of `selected`, chosen from the Flow Bar; None detects among them (LANG-05).
    pub active: Option<String>,
}

impl LanguageSettings {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(unknown) = self.selected.iter().find(|id| !catalog::is_known(id)) {
            return Err(t!("error.languages.unknown", language = unknown));
        }
        for (index, id) in self.selected.iter().enumerate() {
            let clash = self.selected[..index].iter().any(|other| {
                other == id
                    || catalog::group(other).is_some_and(|group| Some(group) == catalog::group(id))
            });
            if clash {
                return Err(t!("error.languages.oneVariant", language = id));
            }
        }
        if let Some(active) = &self.active
            && !self.selected.contains(active)
        {
            return Err(t!("error.languages.activeNotSelected"));
        }
        Ok(())
    }

    /// Checks a choice from the Languages dialog.
    pub fn validate_choice(&self) -> Result<(), String> {
        if !self.auto_detect && self.selected.is_empty() {
            return Err(t!("error.languages.noneSelected"));
        }
        self.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn languages(auto_detect: bool, selected: &[&str]) -> LanguageSettings {
        LanguageSettings {
            auto_detect,
            selected: selected.iter().map(|id| (*id).to_owned()).collect(),
            active: None,
        }
    }

    #[test]
    fn a_choice_needs_a_language_or_auto_detect() {
        assert_eq!(
            languages(false, &[]).validate_choice().unwrap_err(),
            "Please select at least one language or turn on auto-detect"
        );
        languages(true, &[]).validate_choice().unwrap();
        languages(false, &["en-GB", "fr"])
            .validate_choice()
            .unwrap();
    }

    #[test]
    fn variants_of_one_language_exclude_each_other() {
        assert!(languages(false, &["en-GB", "en-US"]).validate().is_err());
        assert!(
            languages(false, &["zh-Hans", "zh-Hant"])
                .validate()
                .is_err()
        );
        languages(false, &["zh-Hant", "yue"]).validate().unwrap();
        assert!(languages(false, &["klingon"]).validate().is_err());
        let mut active = languages(false, &["fr"]);
        active.active = Some("de".into());
        assert!(active.validate().is_err());
    }
}
