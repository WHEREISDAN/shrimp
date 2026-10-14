//! The first "press enter" with the voice command off (CORE-17): the phrase
//! is left out and a one-time prompt offers Great or Disable.

mod support;

use shrimp_lib::notify::NoticeAction;
use support::*;

fn saying(transcript: &str) -> Harness {
    Harness::with(
        5.0,
        0.3,
        fast_timing(),
        Setup {
            transcript: Some(transcript.into()),
            ..Setup::default()
        },
    )
}

fn idle(h: &Harness) {
    wait_until("idle", || {
        h.fakes.last_state() == Some(DictationState::Idle)
    });
}

fn prompt(h: &Harness) -> Notice {
    wait_until("the prompt", || {
        h.fakes.has_notice("left out “press enter”")
    });
    h.fakes
        .notices
        .lock()
        .unwrap()
        .iter()
        .find(|notice| notice.message.contains("left out “press enter”"))
        .cloned()
        .unwrap()
}

#[test]
fn the_first_press_enter_is_left_out_and_asks_once() {
    let h = saying("Ship it, press enter.");
    dictate(&h);
    let asked = prompt(&h);
    assert_eq!(
        asked.actions,
        [
            NoticeAction::EnablePressEnter,
            NoticeAction::DisablePressEnter
        ]
    );
    assert_eq!(h.fakes.inserted(), ["Ship it"]);
    assert!(h.fakes.pressed.lock().unwrap().is_empty(), "no Enter yet");
    let experimental = h.settings.get().experimental;
    assert!(experimental.press_enter_prompt_seen);
    assert!(
        !experimental.press_enter_command,
        "ignoring it leaves it off"
    );

    idle(&h);
    dictate(&h);
    wait_until("the second insert", || h.fakes.inserted().len() == 2);
    assert_eq!(h.fakes.inserted()[1], "Ship it, press enter.");
    let prompts = h
        .fakes
        .messages()
        .iter()
        .filter(|message| message.contains("left out “press enter”"))
        .count();
    assert_eq!(prompts, 1);
}

#[test]
fn great_turns_the_command_on_and_presses_enter() {
    let h = saying("Ship it, press enter.");
    dictate(&h);
    prompt(&h);
    idle(&h);
    h.controller
        .send(ControllerInput::AnswerPressEnter { enable: true });
    wait_until("enter", || !h.fakes.pressed.lock().unwrap().is_empty());
    assert_eq!(
        h.fakes.pressed.lock().unwrap().as_slice(),
        [(SyntheticKey::Enter, Modifiers::empty())]
    );
    assert!(h.settings.get().experimental.press_enter_command);

    dictate(&h);
    wait_until("the next enter", || {
        h.fakes.pressed.lock().unwrap().len() == 2
    });
    assert_eq!(h.fakes.inserted(), ["Ship it", "Ship it"]);
}

#[test]
fn disable_types_the_words_back_and_keeps_them_as_text() {
    let h = saying("Ship it, press enter.");
    dictate(&h);
    prompt(&h);
    idle(&h);
    h.controller
        .send(ControllerInput::AnswerPressEnter { enable: false });
    wait_until("the words back", || h.fakes.inserted().len() == 2);
    assert_eq!(h.fakes.inserted(), ["Ship it", " press enter"]);
    assert!(h.fakes.pressed.lock().unwrap().is_empty());
    assert!(!h.settings.get().experimental.press_enter_command);
}

#[test]
fn press_enter_alone_pastes_nothing_and_asks() {
    let h = saying("Press enter.");
    dictate(&h);
    prompt(&h);
    idle(&h);
    assert!(h.fakes.inserted().is_empty());
    assert!(h.fakes.pressed.lock().unwrap().is_empty());
    assert!(
        !h.fakes.has_notice("Nothing to insert"),
        "{:?}",
        h.fakes.messages()
    );
}

#[test]
fn mid_sentence_press_enter_stays_text_and_asks_nothing() {
    let h = saying("Did you press enter? Yes.");
    dictate(&h);
    wait_until("insertion", || !h.fakes.inserted().is_empty());
    idle(&h);
    assert_eq!(h.fakes.inserted(), ["Did you press enter? Yes."]);
    assert!(!h.settings.get().experimental.press_enter_prompt_seen);
}
