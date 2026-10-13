//! "Speaking a different language?" (LANG-04): after a dictation in fixed
//! languages, Whisper checks what was spoken, and a confident other language
//! gets a card at most once a day.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use super::Controller;
use crate::audio::{AudioClip, level};
use crate::dictation::state::ControllerInput;
use crate::lang;
use crate::notify::{Notice, NoticeAction, NoticeCategory};
use crate::t;

/// Only a clear guess counts.
const CONFIDENT: f32 = 0.75;
const SECONDS_PER_DAY: u64 = 86_400;

fn today() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    (seconds / SECONDS_PER_DAY).to_string()
}

/// The display name of a Whisper code, in the interface language.
fn language_name(code: &str) -> String {
    let id = lang::catalog::from_locale(code).unwrap_or_else(|| code.to_owned());
    let english = lang::catalog::languages()
        .into_iter()
        .find(|language| language.id == id)
        .map(|language| {
            language
                .name
                .split(" (")
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .unwrap_or_else(|| code.to_owned());
    crate::i18n::language_name(code, &english)
}

impl Controller {
    /// Checks the dictation's language in the background.
    pub(super) fn check_language(&self, clip: AudioClip) {
        let engine = Arc::clone(&self.pipeline.engine);
        let tx = self.tx.clone();
        tauri::async_runtime::spawn(async move {
            let Some(speech) = level::trim_silence(&clip) else {
                return;
            };
            let speech = level::normalize(&speech);
            if let Some((code, probability)) = engine.detect_language(&speech).await {
                let _ = tx.send(ControllerInput::LanguageDetected { code, probability });
            }
        });
    }

    pub(super) fn language_detected(&mut self, code: String, probability: f32) {
        let settings = self.settings.get();
        let languages = &settings.languages;
        let yours = languages
            .selected
            .iter()
            .any(|id| lang::catalog::whisper_code(id) == code);
        log::info!("the dictation sounded like {code} ({probability:.2})");
        if languages.auto_detect || yours || probability < CONFIDENT {
            return;
        }
        let notices = &settings.notices;
        let today = today();
        if notices.language_card_off || notices.language_card_day == today {
            return;
        }
        let patch = json!({"notices": {"languageCardDay": today}});
        if let Err(error) = self.settings.update(&patch) {
            log::warn!("could not note the language card: {error}");
        }
        let language = language_name(&code);
        self.ui.notice(
            Notice::info(t!("notice.language.detected", language = language))
                .with_action(NoticeAction::OpenLanguageSettings)
                .with_action(NoticeAction::AddDetectedLanguage)
                .with_action(NoticeAction::DismissLanguageCard)
                .in_category(NoticeCategory::Suggestion),
        );
        self.detected_language = Some(code);
    }

    /// The card's Add language: the language joins the user's.
    pub(super) fn add_detected_language(&mut self) {
        let Some(code) = self.detected_language.take() else {
            return;
        };
        let id = lang::catalog::from_locale(&code).unwrap_or(code);
        let mut selected = self.settings.get().languages.selected;
        let group = lang::catalog::group(&id);
        if selected
            .iter()
            .any(|other| *other == id || (group.is_some() && lang::catalog::group(other) == group))
        {
            return;
        }
        selected.push(id);
        if let Err(error) = self
            .settings
            .update(&json!({"languages": {"selected": selected}}))
        {
            log::warn!("could not add the language: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_languages_without_their_variant() {
        assert_eq!(language_name("fr"), "French");
        assert_eq!(language_name("en"), "English");
        assert_eq!(language_name("zh"), "Chinese");
    }
}
