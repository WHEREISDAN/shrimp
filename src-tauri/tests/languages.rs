//! Languages through the controller (LANG-02..04): what the engine is asked
//! for, variant spelling, and the different-language card.

mod support;

use shrimp_lib::notify::NoticeAction;
use support::*;

fn with_languages(selected: &[&str], auto_detect: bool, active: Option<&str>) -> Harness {
    let mut settings = Settings::default();
    settings.languages.selected = selected.iter().map(|id| (*id).to_owned()).collect();
    settings.languages.auto_detect = auto_detect;
    settings.languages.active = active.map(str::to_owned);
    Harness::with(
        1.0,
        0.3,
        fast_timing(),
        Setup {
            settings,
            ..Setup::default()
        },
    )
}

fn asked(h: &Harness) -> Vec<String> {
    h.fakes.languages.lock().unwrap().last().cloned().unwrap()
}

#[test]
fn the_engine_detects_among_the_selected_languages_or_any_with_auto_detect() {
    let h = with_languages(&["en-GB", "fr", "de-CH"], false, None);
    dictate(&h);
    wait_until("inserted", || !h.fakes.inserted().is_empty());
    assert_eq!(asked(&h), ["en", "fr", "de"]);

    let auto = with_languages(&["fr"], true, None);
    dictate(&auto);
    wait_until("inserted", || !auto.fakes.inserted().is_empty());
    assert!(asked(&auto).is_empty());

    let chosen = with_languages(&["en-GB", "fr"], false, Some("fr"));
    dictate(&chosen);
    wait_until("inserted", || !chosen.fakes.inserted().is_empty());
    assert_eq!(asked(&chosen), ["fr"], "the Flow Bar's choice");
}

#[test]
fn british_english_is_spelled_british() {
    let h = with_languages(&["en-GB", "fr"], false, None);
    *h.fakes.transcript.lock().unwrap() = Some("My favorite color is gray.".into());
    dictate(&h);
    wait_until("inserted", || !h.fakes.inserted().is_empty());
    assert_eq!(h.fakes.inserted(), ["My favourite colour is grey."]);

    let swiss = with_languages(&["de-CH"], false, None);
    *swiss.fakes.transcript.lock().unwrap() = Some("Die Straße ist groß.".into());
    *swiss.fakes.spoken.lock().unwrap() = Some("de".into());
    dictate(&swiss);
    wait_until("inserted", || !swiss.fakes.inserted().is_empty());
    assert_eq!(swiss.fakes.inserted(), ["Die Strasse ist gross."]);
}

#[test]
fn another_language_shows_the_card_once_a_day_and_add_language_adds_it() {
    let h = with_languages(&["en-US"], false, None);
    *h.fakes.detected.lock().unwrap() = Some(("fr".into(), 0.93));
    dictate(&h);
    wait_until("card", || {
        h.fakes.has_notice("Speaking a different language?")
    });
    let card = h
        .fakes
        .notices
        .lock()
        .unwrap()
        .iter()
        .find(|notice| notice.message.contains("Speaking a different language?"))
        .cloned()
        .unwrap();
    assert_eq!(
        card.message,
        "Speaking a different language? That sounded like French."
    );
    assert_eq!(
        card.actions,
        [
            NoticeAction::OpenLanguageSettings,
            NoticeAction::AddDetectedLanguage,
            NoticeAction::DismissLanguageCard
        ]
    );
    h.controller.send(ControllerInput::AddDetectedLanguage);
    wait_until("added", || {
        h.settings.get().languages.selected == ["en-US", "fr"]
    });

    *h.fakes.detected.lock().unwrap() = Some(("es".into(), 0.95));
    dictate(&h);
    wait_until("second insert", || h.fakes.inserted().len() == 2);
    std::thread::sleep(Duration::from_millis(300));
    let cards = h
        .fakes
        .messages()
        .iter()
        .filter(|message| message.contains("Speaking a different language?"))
        .count();
    assert_eq!(cards, 1, "at most once a day");
}

#[test]
fn no_card_for_your_languages_unsure_guesses_or_auto_detect() {
    let h = with_languages(&["en-US", "fr"], false, None);
    *h.fakes.detected.lock().unwrap() = Some(("fr".into(), 0.99));
    dictate(&h);
    wait_until("inserted", || !h.fakes.inserted().is_empty());
    *h.fakes.detected.lock().unwrap() = Some(("es".into(), 0.4));
    dictate(&h);
    wait_until("second", || h.fakes.inserted().len() == 2);
    std::thread::sleep(Duration::from_millis(300));
    assert!(!h.fakes.has_notice("Speaking a different language?"));

    let auto = with_languages(&[], true, None);
    *auto.fakes.detected.lock().unwrap() = Some(("es".into(), 0.99));
    dictate(&auto);
    wait_until("inserted", || !auto.fakes.inserted().is_empty());
    std::thread::sleep(Duration::from_millis(300));
    assert!(!auto.fakes.has_notice("Speaking a different language?"));
}
