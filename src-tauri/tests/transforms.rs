//! Transforms through the controller: Polish on the selection, the field or
//! the last dictation, the changes view's Copy, Retry and Undo, the limits
//! and errors, and auto-apply after a dictation.

mod support;

use std::time::Duration;

use shrimp_lib::notify::NoticeAction;
use shrimp_lib::platform::Editable;
use support::*;

fn transforms_on() -> Settings {
    let mut settings = Settings::default();
    settings.transforms.enabled = true;
    settings
}

fn selection(text: &str) -> FieldContext {
    FieldContext {
        field_id: 7,
        editable: Editable::Yes,
        selection: Some(TextRange {
            start: 0,
            len: text.encode_utf16().count(),
        }),
        selected_text: text.into(),
        ..FieldContext::default()
    }
}

fn cursor_in(text: &str) -> FieldContext {
    FieldContext {
        field_id: 7,
        editable: Editable::Yes,
        selection: Some(TextRange {
            start: text.encode_utf16().count(),
            len: 0,
        }),
        text: Some(text.into()),
        length: text.encode_utf16().count(),
        ..FieldContext::default()
    }
}

fn harness(field: Option<FieldContext>, replies: Vec<Result<String, LlmError>>) -> Harness {
    Harness::with(
        5.0,
        0.3,
        fast_timing(),
        Setup {
            settings: transforms_on(),
            app: Some(app("com.apple.TextEdit", "TextEdit")),
            field,
            ..ai(CleanupLevel::None, replies)
        },
    )
}

fn press(h: &Harness, action: Action) {
    h.shortcut(action, true);
    h.shortcut(action, false);
}

fn idle(h: &Harness) {
    wait_until("idle", || {
        h.fakes.last_state() == Some(DictationState::Idle)
    });
}

#[test]
fn polish_rewrites_the_selection_in_place() {
    let h = harness(
        Some(selection("teh report is late")),
        vec![Ok("The report is late.".into())],
    );
    press(&h, Action::Polish);
    wait_until("insertion", || {
        h.fakes.inserted() == ["The report is late."]
    });
    let prompt = h.fakes.prompts.lock().unwrap()[0].clone();
    assert!(prompt.system.contains("You polish text"));
    assert_eq!(
        prompt.messages[0].content,
        "<text>teh report is late</text>"
    );
    wait_until("the notice", || h.fakes.has_notice("Done."));
    let notice = h.fakes.notices.lock().unwrap().last().unwrap().clone();
    assert_eq!(notice.action(), Some(NoticeAction::ViewTransformChanges));

    h.controller
        .send(ControllerInput::Transform(TransformAction::View));
    wait_until("the changes", || {
        !h.fakes.changes.lock().unwrap().is_empty()
    });
    let view = h.fakes.changes.lock().unwrap()[0].clone();
    assert_eq!(view.name, "Polish");
    assert_eq!(view.original, "teh report is late");
    assert_eq!(view.result, "The report is late.");
}

#[test]
fn with_nothing_selected_the_whole_field_is_transformed() {
    let h = harness(
        Some(cursor_in("draft text here")),
        vec![Ok("Final text.".into())],
    );
    press(&h, Action::PromptEngineer);
    wait_until("insertion", || h.fakes.inserted() == ["Final text."]);
    assert_eq!(
        h.fakes.selected_ranges.lock().unwrap().as_slice(),
        [TextRange { start: 0, len: 15 }]
    );
    let prompt = h.fakes.prompts.lock().unwrap()[0].clone();
    assert!(prompt.system.contains("prompt for an AI assistant"));
}

#[test]
fn an_empty_field_takes_the_last_dictation_of_the_past_minute() {
    let h = harness(
        Some(cursor_in("")),
        vec![Ok("Hello from the shrimp team.".into())],
    );
    dictate(&h);
    wait_until("the dictation", || h.fakes.inserted() == [TEXT]);
    idle(&h);
    press(&h, Action::Polish);
    wait_until("the transform", || {
        h.fakes.inserted() == [TEXT, "Hello from the shrimp team."]
    });
    let prompt = h.fakes.prompts.lock().unwrap()[0].clone();
    assert_eq!(prompt.messages[0].content, format!("<text>{TEXT}</text>"));
}

#[test]
fn no_text_and_too_much_text_give_the_documented_errors() {
    let h = harness(Some(cursor_in("")), vec![Ok("unused".into())]);
    press(&h, Action::Polish);
    wait_until("the error", || {
        h.fakes.has_notice("Select text to apply a transform")
    });

    let long = "word ".repeat(1_001);
    let h = harness(Some(selection(&long)), vec![Ok("unused".into())]);
    press(&h, Action::Polish);
    wait_until("the error", || {
        h.fakes
            .has_notice("Oops, too long to polish. Try again with under 1000 words.")
    });
    assert!(h.fakes.prompts.lock().unwrap().is_empty());
}

#[test]
fn a_timeout_or_an_empty_reply_changes_nothing() {
    let h = harness(Some(selection("keep this")), vec![Err(LlmError::Timeout)]);
    press(&h, Action::Polish);
    wait_until("the error", || h.fakes.has_notice("Transform timed out"));
    let h = harness(Some(selection("keep this")), vec![Ok("  ".into())]);
    press(&h, Action::Polish);
    wait_until("the error", || {
        h.fakes.has_notice("Transform returned no text")
    });
    assert!(h.fakes.inserted().is_empty());
}

#[test]
fn undo_puts_the_original_back_where_the_result_is() {
    let h = harness(Some(selection("teh plan")), vec![Ok("The plan.".into())]);
    press(&h, Action::Polish);
    wait_until("insertion", || h.fakes.inserted() == ["The plan."]);
    idle(&h);
    *h.fakes.field_later.lock().unwrap() = Some(cursor_in("The plan."));
    h.controller
        .send(ControllerInput::Transform(TransformAction::Undo));
    wait_until("the undo", || {
        h.fakes.inserted() == ["The plan.", "teh plan"]
    });
    assert_eq!(
        h.fakes.selected_ranges.lock().unwrap().last(),
        Some(&TextRange { start: 0, len: 9 })
    );
    wait_until("the notice", || h.fakes.has_notice("Undone"));
}

#[test]
fn undo_after_the_text_changed_leaves_the_original_on_the_clipboard() {
    let h = harness(Some(selection("teh plan")), vec![Ok("The plan.".into())]);
    press(&h, Action::Polish);
    wait_until("insertion", || h.fakes.inserted() == ["The plan."]);
    idle(&h);
    *h.fakes.field_later.lock().unwrap() = Some(cursor_in("Someone edited this."));
    h.controller
        .send(ControllerInput::Transform(TransformAction::Undo));
    wait_until("the copy", || {
        h.fakes.copied.lock().unwrap().as_slice() == ["teh plan"]
    });
    assert_eq!(h.fakes.inserted(), ["The plan."]);
}

#[test]
fn retry_swaps_in_a_new_result_and_copy_copies_it() {
    let h = harness(
        Some(selection("teh plan")),
        vec![Ok("The plan.".into()), Ok("Our plan.".into())],
    );
    press(&h, Action::Polish);
    wait_until("insertion", || h.fakes.inserted() == ["The plan."]);
    idle(&h);
    *h.fakes.field_later.lock().unwrap() = Some(cursor_in("The plan."));
    h.controller
        .send(ControllerInput::Transform(TransformAction::Retry));
    wait_until("the retry", || {
        h.fakes.inserted() == ["The plan.", "Our plan."]
    });
    wait_until("the changes", || {
        h.fakes
            .changes
            .lock()
            .unwrap()
            .last()
            .is_some_and(|view| view.result == "Our plan." && view.original == "teh plan")
    });
    idle(&h);
    h.controller
        .send(ControllerInput::Transform(TransformAction::Copy));
    wait_until("the copy", || {
        h.fakes.copied.lock().unwrap().as_slice() == ["Our plan."]
    });
}

#[test]
fn transforms_wait_for_a_dictation_to_finish() {
    let h = harness(Some(selection("text")), vec![Ok("Text.".into())]);
    h.shortcut(Action::PushToTalk, true);
    std::thread::sleep(Duration::from_millis(200));
    press(&h, Action::Polish);
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        h.fakes.prompts.lock().unwrap().is_empty(),
        "ignored while recording"
    );
    h.shortcut(Action::PushToTalk, false);
}

#[test]
fn an_incomplete_custom_transform_does_nothing() {
    let mut settings = transforms_on();
    settings
        .transforms
        .custom
        .push(shrimp_lib::settings::CustomTransform {
            id: "draft".into(),
            shortcut: Some("Opt+3".into()),
            ..Default::default()
        });
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        Setup {
            settings,
            field: Some(selection("text")),
            ..ai(CleanupLevel::None, vec![Ok("Text.".into())])
        },
    );
    press(&h, Action::CustomTransform(0));
    std::thread::sleep(Duration::from_millis(300));
    assert!(h.fakes.prompts.lock().unwrap().is_empty());
    assert!(h.fakes.messages().is_empty());
}

fn auto_polish() -> Settings {
    let mut settings = transforms_on();
    settings.transforms.auto_apply = Some("polish".into());
    settings
}

#[test]
fn auto_apply_runs_after_a_dictation_and_falls_back_on_failure() {
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        Setup {
            settings: auto_polish(),
            ..ai(
                CleanupLevel::None,
                vec![Ok("Hello from the Shrimp team!".into())],
            )
        },
    );
    dictate(&h);
    wait_until("insertion", || {
        h.fakes.inserted() == ["Hello from the Shrimp team!"]
    });

    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        Setup {
            settings: auto_polish(),
            ..ai(
                CleanupLevel::None,
                vec![Err(LlmError::Offline("down".into()))],
            )
        },
    );
    dictate(&h);
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
    wait_until("the notice", || h.fakes.has_notice("Polish didn't run"));
}
