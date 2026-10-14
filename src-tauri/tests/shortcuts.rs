//! Shortcut recording through the controller (KEYS-01..07), another app's
//! shortcut during a held dictation (CORE-12), the cancel notice's Undo
//! (CORE-11) and Secure Keyboard Entry (KEYS-09), with fake hooks.

mod support;

use shrimp_lib::dictation::bindings;
use shrimp_lib::notify::NoticeAction;
use support::*;

fn trigger(text: &str) -> Trigger {
    text.parse().unwrap()
}

fn record(h: &Harness, action: Action, index: Option<usize>, text: &str, both_sides: bool) {
    h.controller
        .send(ControllerInput::RecordShortcut { action, index });
    wait_until("recording", || *h.fakes.recording_shortcut.lock().unwrap());
    h.controller
        .send(ControllerInput::Recording(RecordUpdate::Done(Recorded {
            trigger: trigger(text),
            both_sides,
        })));
    wait_until("recording ends", || {
        !*h.fakes.recording_shortcut.lock().unwrap()
    });
}

fn last_status(h: &Harness) -> RecordingStatus {
    h.fakes
        .recording_statuses
        .lock()
        .unwrap()
        .last()
        .cloned()
        .unwrap()
}

#[test]
fn a_recorded_shortcut_is_saved_and_bound() {
    let h = Harness::speech(1.0);
    record(&h, Action::PushToTalk, None, "Mouse4", false);
    assert_eq!(
        last_status(&h),
        RecordingStatus::Saved {
            action: Action::PushToTalk,
            shortcut: "Mouse 4".into(),
            warning: None,
        }
    );
    assert_eq!(
        h.settings.get().shortcuts.bindings["pushToTalk"],
        ["Fn", "Mouse4"]
    );
    wait_until("rebound", || {
        h.fakes
            .bound
            .lock()
            .unwrap()
            .iter()
            .any(|binding| binding.trigger == trigger("Mouse4"))
    });
}

#[test]
fn rejected_shortcuts_leave_the_binding_unchanged() {
    let h = Harness::speech(1.0);
    record(&h, Action::PressEnter, None, "Cmd+V", false);
    assert_eq!(
        last_status(&h),
        RecordingStatus::Failed {
            action: Action::PressEnter,
            message: "This shortcut is not allowed: ⌘V".into(),
        }
    );
    record(&h, Action::PushToTalk, Some(0), "Cmd+Cmd", true);
    assert_eq!(
        last_status(&h),
        RecordingStatus::Failed {
            action: Action::PushToTalk,
            message: "Shortcut cannot contain both left and right version of the same key".into(),
        }
    );
    record(&h, Action::HandsFree, Some(0), "Cmd+Ctrl+V", false);
    assert!(matches!(
        last_status(&h),
        RecordingStatus::Failed { message, .. } if message == "This shortcut is already in use"
    ));
    assert!(h.settings.get().shortcuts.bindings.is_empty());
}

#[test]
fn a_hands_free_shortcut_inside_push_to_talk_saves_with_a_warning() {
    let h = Harness::speech(1.0);
    record(&h, Action::PushToTalk, Some(0), "Ctrl+Opt+Space", false);
    record(&h, Action::HandsFree, Some(0), "Ctrl+Opt", false);
    let RecordingStatus::Saved { warning, .. } = last_status(&h) else {
        panic!("not saved");
    };
    assert_eq!(
        warning.as_deref(),
        Some(
            "Warning: Your Hands-free mode shortcut is a part of a Push-to-talk shortcut. \
             This may lead to conflicts."
        )
    );
}

#[test]
fn shortcuts_do_nothing_while_one_is_recorded_and_cancelling_restores_them() {
    let h = Harness::speech(1.0);
    h.controller.send(ControllerInput::RecordShortcut {
        action: Action::PushToTalk,
        index: Some(0),
    });
    wait_until("recording", || *h.fakes.recording_shortcut.lock().unwrap());
    h.controller.send(ControllerInput::CancelRecording);
    wait_until("cancelled", || {
        matches!(last_status(&h), RecordingStatus::Cancelled { .. })
    });
    assert!(!*h.fakes.recording_shortcut.lock().unwrap());
    assert!(h.settings.get().shortcuts.bindings.is_empty());
}

#[test]
fn another_shortcut_early_in_a_held_dictation_cancels_it() {
    let h = Harness::speech(3.0);
    h.shortcut(Action::PushToTalk, true);
    wait_until("recording", || {
        matches!(h.fakes.last_state(), Some(DictationState::Recording { .. }))
    });
    h.controller.send(ControllerInput::OtherKey);
    wait_until("cancelled", || h.fakes.has_notice("another shortcut"));
    h.shortcut(Action::PushToTalk, false);
    std::thread::sleep(Duration::from_millis(300));
    assert!(h.fakes.inserted().is_empty());
    assert_eq!(h.fakes.last_state(), Some(DictationState::Idle));
}

#[test]
fn another_key_after_the_first_second_is_ignored() {
    let h = Harness::speech(3.0);
    h.shortcut(Action::PushToTalk, true);
    std::thread::sleep(Duration::from_millis(1100));
    h.controller.send(ControllerInput::OtherKey);
    std::thread::sleep(Duration::from_millis(100));
    h.shortcut(Action::PushToTalk, false);
    wait_until("inserted", || !h.fakes.inserted().is_empty());
    assert!(!h.fakes.has_notice("another shortcut"));
}

#[test]
fn the_held_shortcut_is_protected_from_stuck_key_recovery_while_recording() {
    let h = Harness::speech(3.0);
    h.shortcut(Action::PushToTalk, true);
    wait_until("protected", || {
        h.fakes
            .protected
            .lock()
            .unwrap()
            .contains(&Action::PushToTalk)
    });
    h.shortcut(Action::PushToTalk, false);
    wait_until("unprotected", || {
        h.fakes.protected.lock().unwrap().is_empty()
    });
}

#[test]
fn undo_on_the_cancel_notice_transcribes_and_inserts_the_cancelled_audio() {
    let h = Harness::speech(3.0);
    h.shortcut(Action::PushToTalk, true);
    std::thread::sleep(Duration::from_millis(700));
    h.shortcut(Action::Cancel, true);
    h.shortcut(Action::Cancel, false);
    h.shortcut(Action::PushToTalk, false);
    wait_until("cancel notice", || {
        h.fakes.has_notice("Dictation cancelled")
    });
    let notice = h
        .fakes
        .notices
        .lock()
        .unwrap()
        .iter()
        .find(|notice| notice.message == "Dictation cancelled")
        .cloned()
        .unwrap();
    assert_eq!(notice.action(), Some(NoticeAction::UndoCancel));
    std::thread::sleep(Duration::from_millis(200));
    h.controller.send(ControllerInput::UndoCancel);
    wait_until("inserted", || !h.fakes.inserted().is_empty());
    assert_eq!(h.fakes.inserted(), [TEXT]);
}

#[test]
fn secure_keyboard_entry_is_explained() {
    let h = Harness::speech(1.0);
    h.controller
        .send(ControllerInput::SecureInput(Some("1Password".into())));
    wait_until("notice", || {
        h.fakes
            .has_notice("Secure Keyboard Entry is on (macOS names 1Password)")
    });
    assert_eq!(h.controller.secure_input().as_deref(), Some("1Password"));
    h.controller
        .send(ControllerInput::SecureInput(Some("TextEdit".into())));
    wait_until("owner", || {
        h.controller.secure_input().as_deref() == Some("TextEdit")
    });
    assert!(
        !h.fakes.has_notice("macOS names TextEdit"),
        "one notice while it stays on"
    );
    h.controller.send(ControllerInput::SecureInput(None));
    wait_until("cleared", || h.controller.secure_input().is_none());
}

#[test]
fn without_an_fn_key_the_defaults_use_ctrl_opt() {
    let settings = Settings::default();
    let defaults = macos_defaults(false);
    let labels: Vec<String> = [Action::PushToTalk, Action::HandsFree, Action::CommandMode]
        .into_iter()
        .map(|action| macos_label(&bindings::triggers(action, &defaults, &settings)[0]))
        .collect();
    assert_eq!(labels, ["⌃⌥", "⌃⌥Space", "⌃⌥⌘"]);
}

/// KEYS-02, KEYS-11: Hands-free needs no binding of its own, because
/// double-tapping push to talk starts it too.
#[test]
fn hands_free_can_lose_every_binding_and_stays_unbound_after_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let defaults = macos_defaults(true);
    let store = SettingsStore::load(path.clone());
    store
        .update(&serde_json::json!({
            "shortcuts": {"bindings": {"handsFree": ["Fn+Space", "Mouse5"]}}
        }))
        .unwrap();
    for _ in 0..2 {
        let patch =
            bindings::without_trigger(&defaults, &store.get(), Action::HandsFree, 0).unwrap();
        store.update(&patch).unwrap();
    }
    drop(store);

    let settings = SettingsStore::load(path).get();
    assert!(settings.shortcuts.bindings["handsFree"].is_empty());
    assert!(bindings::triggers(Action::HandsFree, &defaults, &settings).is_empty());
    assert!(
        bindings::configured(&defaults, &settings)
            .iter()
            .all(|binding| binding.action != Action::HandsFree)
    );
}

#[test]
fn double_tap_starts_hands_free_without_a_hands_free_binding() {
    let mut settings = Settings::default();
    settings
        .shortcuts
        .bindings
        .insert("handsFree".into(), Vec::new());
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        Setup {
            settings,
            ..Setup::default()
        },
    );
    h.hold(Action::PushToTalk, Duration::from_millis(80));
    std::thread::sleep(Duration::from_millis(100));
    h.hold(Action::PushToTalk, Duration::from_millis(80));
    wait_until("hands-free", || {
        h.fakes.last_state()
            == Some(DictationState::Recording {
                mode: Mode::Locked,
                command: false,
            })
    });
    assert!(
        h.fakes
            .bound
            .lock()
            .unwrap()
            .iter()
            .all(|binding| binding.action != Action::HandsFree)
    );
    // A third quick press would cancel (CORE-05).
    std::thread::sleep(Duration::from_millis(900));
    h.hold(Action::PushToTalk, Duration::from_millis(50));
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
}
