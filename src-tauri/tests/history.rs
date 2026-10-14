//! History through the dictation controller: what each dictation stores,
//! retries, paste last of a failed dictation, cancelled dictations (HIS-10)
//! and the crash journal.

mod support;

use shrimp_lib::notify::NoticeAction;
use support::*;

fn harness(seconds: f32, amplitude: f32, mut setup: Setup) -> Harness {
    setup.history = true;
    Harness::with(seconds, amplitude, fast_timing(), setup)
}

fn list(h: &Harness) -> HistoryList {
    h.history.as_ref().unwrap().list("").unwrap()
}

fn wait_for_entries(h: &Harness, count: usize) {
    wait_until("history entries", || list(h).entries.len() == count);
}

#[test]
fn a_dictation_is_stored_with_its_recording_and_counted() {
    let h = harness(6.0, 0.3, Setup::default());
    dictate(&h);
    wait_for_entries(&h, 1);
    let entry = &list(&h).entries[0];
    assert_eq!(entry.entry.status, HistoryStatus::Inserted);
    assert_eq!(entry.entry.final_text, TEXT);
    assert_eq!(entry.entry.app_name.as_deref(), Some("Slack"));
    assert_eq!(entry.entry.language.as_deref(), Some("en"));
    assert!(entry.has_audio);
    assert!(entry.entry.audio_ms >= 600, "{}", entry.entry.audio_ms);
}

#[test]
fn a_failed_ai_edit_is_marked_unformatted() {
    let setup = ai(
        CleanupLevel::Light,
        vec![Err(LlmError::Unavailable("overloaded".into()))],
    );
    let h = harness(6.0, 0.3, setup);
    dictate(&h);
    wait_for_entries(&h, 1);
    let entry = &list(&h).entries[0].entry;
    assert!(!entry.formatted, "FMT-10");
    assert_eq!(entry.final_text, TEXT);
}

#[test]
fn a_failed_transcription_can_be_retried_from_history() {
    let h = harness(6.0, 0.3, Setup::default());
    h.fakes.failures_left.store(1, Ordering::SeqCst);
    h.shortcut(Action::HandsFree, true);
    h.shortcut(Action::HandsFree, false);
    std::thread::sleep(Duration::from_millis(5_400));
    h.shortcut(Action::HandsFree, true);
    h.shortcut(Action::HandsFree, false);
    wait_for_entries(&h, 1);
    wait_until("idle", || {
        h.fakes.last_state() == Some(DictationState::Idle)
    });
    let failed = list(&h).entries.remove(0);
    assert_eq!(failed.entry.status, HistoryStatus::Failed);
    assert_eq!(failed.retry, Some(RetryKind::Retry), "{failed:?}");
    assert!(h.fakes.inserted().is_empty());

    h.controller
        .send(ControllerInput::RetryHistory(failed.entry.id));
    wait_until("the retry", || {
        list(&h).entries[0].entry.status == HistoryStatus::Transcribed
    });
    let retried = list(&h).entries.remove(0);
    assert_eq!(retried.entry.final_text, TEXT);
    assert!(h.fakes.inserted().is_empty(), "a retry never pastes");
    assert_eq!(list(&h).entries.len(), 1);
}

#[test]
fn paste_last_of_a_failed_dictation_updates_its_entry() {
    let h = harness(6.0, 0.3, Setup::default());
    h.fakes.failures_left.store(1, Ordering::SeqCst);
    dictate(&h);
    wait_for_entries(&h, 1);
    wait_until("idle", || {
        h.fakes.last_state() == Some(DictationState::Idle)
    });
    h.shortcut(Action::PasteLast, true);
    h.shortcut(Action::PasteLast, false);
    wait_until("the paste", || h.fakes.inserted() == [TEXT]);
    wait_until("the update", || {
        list(&h).entries[0].entry.status == HistoryStatus::Inserted
    });
    assert_eq!(list(&h).entries.len(), 1);
}

#[test]
fn retry_is_refused_while_dictating() {
    let h = harness(6.0, 0.3, Setup::default());
    h.shortcut(Action::PushToTalk, true);
    std::thread::sleep(Duration::from_millis(300));
    h.controller.send(ControllerInput::RetryHistory(1));
    wait_until("the refusal", || {
        h.fakes.has_notice("Cannot retry while dictating.")
    });
    h.shortcut(Action::PushToTalk, false);
}

#[test]
fn silence_is_stored_as_an_empty_entry() {
    let h = harness(6.0, 0.0, Setup::default());
    dictate(&h);
    wait_for_entries(&h, 1);
    assert_eq!(list(&h).entries[0].entry.status, HistoryStatus::Empty);
}

#[test]
fn the_recording_is_saved_while_it_happens_and_removed_after() {
    let h = harness(6.0, 0.3, Setup::default());
    let recordings = h._dir.path().join("recordings");
    let journals = || {
        std::fs::read_dir(&recordings)
            .map(|files| {
                files
                    .filter_map(Result::ok)
                    .filter(|file| file.file_name().to_string_lossy().starts_with("active-"))
                    .count()
            })
            .unwrap_or(0)
    };
    h.shortcut(Action::PushToTalk, true);
    wait_until("the journal", || journals() == 1);
    std::thread::sleep(Duration::from_millis(500));
    h.shortcut(Action::PushToTalk, false);
    wait_for_entries(&h, 1);
    wait_until("the journal is gone", || journals() == 0);
}

/// Holds push to talk for `held`, then cancels with Esc.
fn cancel_after(h: &Harness, held: Duration) {
    h.shortcut(Action::PushToTalk, true);
    std::thread::sleep(held);
    h.shortcut(Action::Cancel, true);
    h.shortcut(Action::Cancel, false);
    h.shortcut(Action::PushToTalk, false);
    wait_until("idle", || {
        h.fakes.last_state() == Some(DictationState::Idle)
    });
}

fn notice(h: &Harness, message: &str) -> Notice {
    wait_until(message, || h.fakes.has_notice(message));
    let notices = h.fakes.notices.lock().unwrap();
    notices
        .iter()
        .rev()
        .find(|notice| notice.message == message)
        .cloned()
        .unwrap()
}

#[test]
fn a_cancelled_dictation_is_kept_for_recovery() {
    let h = harness(6.0, 0.3, Setup::default());
    cancel_after(&h, Duration::from_millis(700));
    wait_for_entries(&h, 1);
    let cancelled = list(&h).entries.remove(0);
    assert_eq!(cancelled.entry.status, HistoryStatus::Cancelled);
    assert_eq!(cancelled.entry.app_name.as_deref(), Some("Slack"));
    assert_eq!(cancelled.retry, Some(RetryKind::Recover));
    assert!(cancelled.has_audio);
    let card = notice(&h, "You can recover your cancelled transcript");
    assert_eq!(
        card.actions,
        [NoticeAction::UndoCancel, NoticeAction::OpenHistory]
    );
    assert_eq!(h.settings.get().notices.cancel_recover_count, 1);

    h.controller
        .send(ControllerInput::RetryHistory(cancelled.entry.id));
    wait_until("the recovery", || {
        list(&h).entries[0].entry.status == HistoryStatus::Transcribed
    });
    assert_eq!(list(&h).entries[0].entry.final_text, TEXT);
    assert!(h.fakes.inserted().is_empty(), "recovering never pastes");
}

#[test]
fn undo_of_a_cancelled_dictation_updates_its_entry() {
    let h = harness(6.0, 0.3, Setup::default());
    cancel_after(&h, Duration::from_millis(700));
    wait_for_entries(&h, 1);
    h.controller.send(ControllerInput::UndoCancel);
    wait_until("the paste", || h.fakes.inserted() == [TEXT]);
    wait_until("the update", || {
        list(&h).entries[0].entry.status == HistoryStatus::Inserted
    });
    assert_eq!(list(&h).entries.len(), 1);
    wait_until("idle", || {
        h.fakes.last_state() == Some(DictationState::Idle)
    });

    // The card showed today already; the next cancel shows the usual notice.
    cancel_after(&h, Duration::from_millis(700));
    let pill = notice(&h, "Dictation cancelled");
    assert_eq!(pill.actions, [NoticeAction::UndoCancel]);
    wait_for_entries(&h, 2);
    h.shortcut(Action::PasteLast, true);
    h.shortcut(Action::PasteLast, false);
    wait_until("the second paste", || h.fakes.inserted().len() == 2);
    wait_until("the second update", || {
        list(&h)
            .entries
            .iter()
            .all(|item| item.entry.status == HistoryStatus::Inserted)
    });
    assert_eq!(list(&h).entries.len(), 2);
}

#[test]
fn a_cancel_under_half_a_second_is_not_stored_but_can_be_undone() {
    let h = harness(6.0, 0.3, Setup::default());
    cancel_after(&h, Duration::from_millis(150));
    let pill = notice(&h, "Dictation cancelled");
    assert_eq!(pill.actions, [NoticeAction::UndoCancel]);
    std::thread::sleep(Duration::from_millis(300));
    assert!(list(&h).entries.is_empty());
    h.controller.send(ControllerInput::UndoCancel);
    wait_until("the paste", || h.fakes.inserted() == [TEXT]);
    wait_for_entries(&h, 1);
    assert_eq!(h.settings.get().notices.cancel_recover_count, 0);
}

#[test]
fn nothing_cancelled_is_stored_with_never_store() {
    let mut settings = Settings::default();
    settings.privacy.history = shrimp_lib::settings::HistoryStorage::Never;
    let h = harness(
        6.0,
        0.3,
        Setup {
            settings,
            ..Setup::default()
        },
    );
    cancel_after(&h, Duration::from_millis(700));
    notice(&h, "Dictation cancelled");
    std::thread::sleep(Duration::from_millis(300));
    assert!(list(&h).entries.is_empty());
}

#[test]
fn an_interrupted_start_is_stored_but_paste_last_keeps_the_last_transcript() {
    let h = harness(6.0, 0.3, Setup::default());
    dictate(&h);
    wait_for_entries(&h, 1);
    wait_until("idle", || {
        h.fakes.last_state() == Some(DictationState::Idle)
    });
    h.shortcut(Action::PushToTalk, true);
    std::thread::sleep(Duration::from_millis(600));
    h.controller.send(ControllerInput::OtherKey);
    h.shortcut(Action::PushToTalk, false);
    notice(
        &h,
        "Dictation cancelled because another shortcut was pressed.",
    );
    wait_for_entries(&h, 2);
    assert_eq!(list(&h).entries[0].entry.status, HistoryStatus::Cancelled);

    h.shortcut(Action::PasteLast, true);
    h.shortcut(Action::PasteLast, false);
    wait_until("the paste", || h.fakes.inserted().len() == 2);
    assert_eq!(h.fakes.inserted(), [TEXT, TEXT]);
    assert_eq!(list(&h).entries[0].entry.status, HistoryStatus::Cancelled);
}

#[test]
fn a_cancelled_command_is_not_stored() {
    let mut settings = Settings::default();
    settings.experimental.command_mode = true;
    let h = harness(
        6.0,
        0.3,
        Setup {
            settings,
            ..Setup::default()
        },
    );
    h.shortcut(Action::CommandMode, true);
    std::thread::sleep(Duration::from_millis(700));
    h.shortcut(Action::Cancel, true);
    h.shortcut(Action::Cancel, false);
    h.shortcut(Action::CommandMode, false);
    notice(&h, "Command cancelled");
    std::thread::sleep(Duration::from_millis(300));
    assert!(list(&h).entries.is_empty());
}

#[test]
fn shutting_down_the_controller_releases_the_database() {
    let h = harness(6.0, 0.3, Setup::default());
    dictate(&h);
    wait_for_entries(&h, 1);
    let history = h.history.as_ref().unwrap();
    assert!(Arc::strong_count(history) > 1);
    assert!(h.controller.shutdown(WAIT));
    wait_until("History's last holder", || Arc::strong_count(history) == 1);
    assert!(
        h.controller.shutdown(WAIT),
        "a second shutdown returns at once"
    );
}
