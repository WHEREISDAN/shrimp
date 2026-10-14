//! One-time suggestions after a dictation: English (Canada) in Canada
//! (LANG-07) and a mouse button for dictating (CORE-18).

mod support;

use shrimp_lib::notify::NoticeAction;
use support::*;

const CANADIAN: &str = "Switch to Canadian English?";
const MOUSE: &str = "Your mouse has extra buttons.";

fn harness(region: Option<&str>, selected: &[&str], mouse_buttons: u32) -> Harness {
    let mut settings = Settings::default();
    settings.languages.selected = selected.iter().map(|id| (*id).to_owned()).collect();
    harness_with(region, mouse_buttons, settings)
}

fn harness_with(region: Option<&str>, mouse_buttons: u32, settings: Settings) -> Harness {
    Harness::with(
        1.0,
        0.3,
        fast_timing(),
        Setup {
            settings,
            region: region.map(str::to_owned),
            mouse_buttons,
            ..Setup::default()
        },
    )
}

fn notice(h: &Harness, text: &str) -> Option<Notice> {
    let notices = h.fakes.notices.lock().unwrap();
    notices
        .iter()
        .find(|notice| notice.message.contains(text))
        .cloned()
}

fn count(h: &Harness, text: &str) -> usize {
    h.fakes
        .messages()
        .iter()
        .filter(|message| message.contains(text))
        .count()
}

/// Dictates with push to talk and waits until the result is in.
fn dictate_and_settle(h: &Harness) {
    let before = h.fakes.inserted().len();
    dictate(h);
    wait_until("inserted", || h.fakes.inserted().len() > before);
    std::thread::sleep(Duration::from_millis(300));
}

#[test]
fn canada_with_another_english_is_offered_canadian_english_once() {
    let h = harness(Some("CA"), &["fr", "en-US"], 0);
    dictate_and_settle(&h);
    let suggestion = notice(&h, CANADIAN).expect("the suggestion");
    assert_eq!(
        suggestion.message,
        "Switch to Canadian English? Spellings like colour and centre will come out right."
    );
    assert_eq!(
        suggestion.actions,
        [NoticeAction::SwitchToCanadianEnglish, NoticeAction::NotNow]
    );
    assert_eq!(suggestion.category, NoticeCategory::Suggestion);
    assert!(h.settings.get().notices.canadian_english_shown);

    h.controller.send(ControllerInput::SwitchToCanadianEnglish);
    wait_until("switched", || {
        h.settings.get().languages.selected == ["fr", "en-CA"]
    });

    *h.fakes.transcript.lock().unwrap() = Some("My favorite color is gray.".into());
    dictate_and_settle(&h);
    assert_eq!(h.fakes.inserted()[1], "My favourite colour is grey.");
    assert_eq!(count(&h, CANADIAN), 1, "once ever");
}

#[test]
fn no_canadian_suggestion_elsewhere_for_canadians_or_when_muted() {
    let american = harness(Some("US"), &["en-US"], 0);
    dictate_and_settle(&american);
    assert_eq!(count(&american, CANADIAN), 0);

    let canadian = harness(Some("CA"), &["en-CA"], 0);
    dictate_and_settle(&canadian);
    assert_eq!(count(&canadian, CANADIAN), 0);

    let mut settings = Settings::default();
    settings.languages.selected = vec!["en-GB".into()];
    settings.notifications.suggestions = false;
    let muted = harness_with(Some("CA"), 0, settings);
    dictate_and_settle(&muted);
    assert_eq!(count(&muted, CANADIAN), 0);
    assert!(
        !muted.settings.get().notices.canadian_english_shown,
        "a muted suggestion isn't used up"
    );
}

#[test]
fn a_mouse_with_extra_buttons_is_suggested_once_after_a_keyboard_dictation() {
    let h = harness(None, &["en-US"], 5);
    dictate_and_settle(&h);
    wait_until("the tip", || notice(&h, MOUSE).is_some());
    let tip = notice(&h, MOUSE).unwrap();
    assert_eq!(
        tip.message,
        "Your mouse has extra buttons. Use one to dictate."
    );
    assert_eq!(
        tip.actions,
        [NoticeAction::SetUpMouseButton, NoticeAction::NotNow]
    );
    assert_eq!(tip.category, NoticeCategory::Suggestion);
    assert!(h.settings.get().notices.mouse_buttons_shown);

    dictate_and_settle(&h);
    assert_eq!(count(&h, MOUSE), 1, "once ever");
    assert_eq!(h.fakes.mouse_reads.load(Ordering::SeqCst), 1);
}

#[test]
fn no_mouse_tip_for_basic_mice_bound_buttons_or_flow_bar_dictations() {
    let basic = harness(None, &["en-US"], 3);
    dictate_and_settle(&basic);
    assert_eq!(basic.fakes.mouse_reads.load(Ordering::SeqCst), 1);
    assert_eq!(count(&basic, MOUSE), 0);
    assert!(!basic.settings.get().notices.mouse_buttons_shown);

    let mut settings = Settings::default();
    settings
        .shortcuts
        .bindings
        .insert("handsFree".into(), vec!["Mouse4".into()]);
    let bound = harness_with(None, 5, settings);
    dictate_and_settle(&bound);
    assert_eq!(bound.fakes.mouse_reads.load(Ordering::SeqCst), 0);
    assert_eq!(count(&bound, MOUSE), 0);

    let clicked = harness(None, &["en-US"], 5);
    clicked.controller.bar(BarControl::Start);
    std::thread::sleep(Duration::from_millis(700));
    clicked.controller.bar(BarControl::Stop);
    wait_until("inserted", || !clicked.fakes.inserted().is_empty());
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(clicked.fakes.mouse_reads.load(Ordering::SeqCst), 0);
}

#[test]
fn one_suggestion_per_dictation() {
    let h = harness(Some("CA"), &["en-US"], 5);
    dictate_and_settle(&h);
    assert_eq!(count(&h, CANADIAN), 1);
    assert_eq!(count(&h, MOUSE), 0);
    dictate_and_settle(&h);
    wait_until("the mouse tip", || count(&h, MOUSE) == 1);
}
