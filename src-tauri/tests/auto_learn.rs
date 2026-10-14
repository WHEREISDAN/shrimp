//! Dictionary auto-learn through the controller (DIC-07): a word of the
//! dictation that the user respells is added, with a notice.

mod support;

use std::time::Duration;

use shrimp_lib::notify::NoticeAction;
use shrimp_lib::platform::Editable;
use support::*;

fn field(text: &str) -> FieldContext {
    FieldContext {
        field_id: 9,
        editable: Editable::Yes,
        selection: Some(TextRange {
            start: text.encode_utf16().count(),
            len: 0,
        }),
        before: text.into(),
        text: Some(text.into()),
        length: text.encode_utf16().count(),
        ..FieldContext::default()
    }
}

fn harness(settings: Settings, edited: &str) -> Harness {
    harness_for(
        "Please ask when to review the draft.",
        settings,
        &[
            field(""),
            field("Please ask when to review the draft."),
            field(edited),
        ],
    )
}

fn harness_for(dictated: &str, settings: Settings, fields: &[FieldContext]) -> Harness {
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        Setup {
            settings,
            app: Some(app("com.apple.TextEdit", "TextEdit")),
            transcript: Some(dictated.into()),
            ..Setup::default()
        },
    );
    *h.fakes.field_sequence.lock().unwrap() = fields.to_vec();
    h
}

#[test]
fn a_respelled_word_is_added_with_a_notice() {
    let h = harness(
        Settings::default(),
        "Please ask Nguyen to review the draft.",
    );
    dictate(&h);
    wait_until("the word", || h.library.learned() == ["Nguyen"]);
    wait_until("the notice", || {
        h.fakes.has_notice("Added “Nguyen” to your dictionary")
    });
    let notice = h.fakes.notices.lock().unwrap().last().unwrap().clone();
    assert_eq!(notice.action(), Some(NoticeAction::OpenDictionary));
}

#[test]
fn everyday_words_and_the_switch_off_teach_nothing() {
    let h = harness(Settings::default(), "Please ask them to review the draft.");
    dictate(&h);
    wait_until("insertion", || !h.fakes.inserted().is_empty());
    std::thread::sleep(Duration::from_millis(800));
    assert!(h.library.learned().is_empty());

    let mut settings = Settings::default();
    settings.dictionary.auto_learn = false;
    let h = harness(settings, "Please ask Nguyen to review the draft.");
    dictate(&h);
    wait_until("insertion", || !h.fakes.inserted().is_empty());
    std::thread::sleep(Duration::from_millis(800));
    assert!(h.library.learned().is_empty());
}

#[test]
fn a_field_with_windows_line_breaks_still_finds_the_dictation() {
    // The paste had "\n"; a Windows edit control holds "\r\n".
    let h = harness_for(
        "Please ask when to review the draft. New line. Thanks.",
        Settings::default(),
        &[
            field(""),
            field("Please ask when to review the draft.\r\nThanks."),
            field("Please ask Nguyen to review the draft.\r\nThanks."),
        ],
    );
    dictate(&h);
    wait_until("the word", || h.library.learned() == ["Nguyen"]);
}
