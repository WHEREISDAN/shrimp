//! History through the dictation controller: what each dictation stores,
//! retries, paste last of a failed dictation, and the crash journal.

mod support;

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

#[test]
fn a_cancelled_dictation_is_not_stored() {
    let h = harness(6.0, 0.3, Setup::default());
    h.shortcut(Action::PushToTalk, true);
    std::thread::sleep(Duration::from_millis(600));
    h.shortcut(Action::Cancel, true);
    h.shortcut(Action::Cancel, false);
    h.shortcut(Action::PushToTalk, false);
    wait_until("idle", || {
        h.fakes.last_state() == Some(DictationState::Idle)
    });
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
