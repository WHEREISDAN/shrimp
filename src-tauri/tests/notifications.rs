//! Notification categories (NOT-01), formatting tips (FMT-13) and French
//! typography (FMT-14) through the dictation controller.

mod support;

use support::*;

const BACKTRACK: &str = "Let's do coffee at two, actually three.";

fn backtrack_setup() -> Setup {
    Setup {
        transcript: Some(BACKTRACK.into()),
        ..ai(
            CleanupLevel::Light,
            vec![Ok("Let's do coffee at 3.".into())],
        )
    }
}

#[test]
fn the_first_backtrack_explains_itself_once() {
    let h = Harness::with(5.0, 0.3, fast_timing(), backtrack_setup());
    dictate(&h);
    wait_until("the tip", || {
        h.fakes.has_notice("Shrimp kept only your correction")
    });
    let tip = h
        .fakes
        .notices
        .lock()
        .unwrap()
        .iter()
        .find(|notice| notice.message.contains("kept only"))
        .cloned()
        .unwrap();
    assert_eq!(tip.category, NoticeCategory::Announcement);
    assert_eq!(h.settings.get().notices.formatting_tips.backtrack, 1);

    wait_until("idle", || {
        h.fakes.last_state() == Some(DictationState::Idle)
    });
    dictate(&h);
    wait_until("the second paste", || h.fakes.inserted().len() == 2);
    std::thread::sleep(Duration::from_millis(200));
    let tips = h
        .fakes
        .messages()
        .iter()
        .filter(|message| message.contains("actually"))
        .count();
    assert_eq!(tips, 1, "no reminder the same day");
}

#[test]
fn muted_announcements_show_no_tip_and_count_nothing() {
    let mut setup = backtrack_setup();
    setup.settings.notifications.announcements = false;
    let h = Harness::with(5.0, 0.3, fast_timing(), setup);
    dictate(&h);
    wait_until("insertion", || {
        h.fakes.inserted() == ["Let's do coffee at 3."]
    });
    std::thread::sleep(Duration::from_millis(200));
    assert!(!h.fakes.has_notice("kept only"));
    assert_eq!(h.settings.get().notices.formatting_tips.backtrack, 0);
}

#[test]
fn muted_suggestions_drop_the_short_tap_tip_but_errors_still_show() {
    let mut setup = Setup::default();
    setup.settings.notifications.suggestions = false;
    let h = Harness::with(5.0, 0.3, fast_timing(), setup);
    h.hold(Action::PushToTalk, Duration::from_millis(100));
    std::thread::sleep(Duration::from_millis(900));
    assert!(!h.fakes.has_notice("Hold fn to dictate"));
    h.controller.send(ControllerInput::RetryHistory(42));
    wait_until("the status notice", || {
        h.fakes.has_notice("Retry failed. Please try again.")
    });
}

#[test]
fn french_gets_narrow_no_break_spaces() {
    let setup = Setup {
        transcript: Some("Bonjour ! Tu viens demain ? Voici la liste : pain, lait.".into()),
        ..Setup::default()
    };
    let h = Harness::with(5.0, 0.3, fast_timing(), setup);
    *h.fakes.spoken.lock().unwrap() = Some("fr".into());
    dictate(&h);
    wait_until("insertion", || !h.fakes.inserted().is_empty());
    assert_eq!(
        h.fakes.inserted()[0],
        "Bonjour\u{202F}! Tu viens demain\u{202F}? Voici la liste\u{202F}: pain, lait."
    );
}

#[test]
fn french_in_a_code_editor_is_left_alone() {
    let setup = Setup {
        transcript: Some("Pourquoi ? Parce que !".into()),
        app: Some(app("com.microsoft.VSCode", "Visual Studio Code")),
        ..Setup::default()
    };
    let h = Harness::with(5.0, 0.3, fast_timing(), setup);
    *h.fakes.spoken.lock().unwrap() = Some("fr".into());
    dictate(&h);
    wait_until("insertion", || !h.fakes.inserted().is_empty());
    assert!(
        !h.fakes.inserted()[0].contains('\u{202F}'),
        "{:?}",
        h.fakes.inserted()
    );
}
