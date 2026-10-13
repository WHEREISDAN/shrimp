//! Sound settings (SND-02) and muting or pausing while dictating (SND-03,
//! SND-04), with fake cues and media.

mod support;

use support::*;

fn with(settings: Settings) -> Harness {
    Harness::with(
        1.5,
        0.3,
        fast_timing(),
        Setup {
            settings,
            ..Setup::default()
        },
    )
}

#[test]
fn dictation_sounds_can_be_turned_off() {
    let mut settings = Settings::default();
    settings.sounds.dictation = false;
    let h = with(settings);
    dictate(&h);
    wait_until("inserted", || !h.fakes.inserted().is_empty());
    assert!(h.fakes.cues().is_empty());
}

#[test]
fn warnings_come_with_the_notice_sound_unless_it_is_off() {
    let h = Harness::speech(1.0);
    h.shortcut(Action::PushToTalk, true);
    h.shortcut(Action::PushToTalk, false);
    // A short tap gives an info tip: no sound.
    wait_until("tip", || h.fakes.has_notice("Hold fn"));
    assert!(!h.fakes.cues().contains(&Cue::Notice));
    h.controller
        .send(ControllerInput::SecureInput(Some("1Password".into())));
    wait_until("warning", || h.fakes.cues().contains(&Cue::Notice));

    let mut settings = Settings::default();
    settings.sounds.notifications = false;
    let quiet = with(settings);
    quiet
        .controller
        .send(ControllerInput::SecureInput(Some("1Password".into())));
    wait_until("warning", || {
        quiet.fakes.has_notice("Secure Keyboard Entry")
    });
    assert!(!quiet.fakes.cues().contains(&Cue::Notice));
}

#[test]
fn nothing_is_muted_or_paused_by_default() {
    let h = Harness::speech(1.5);
    dictate(&h);
    wait_until("inserted", || !h.fakes.inserted().is_empty());
    assert!(h.fakes.media.lock().unwrap().is_empty());
}

#[test]
fn muting_starts_within_300_ms_and_is_undone_when_the_dictation_ends() {
    let mut settings = Settings::default();
    settings.media.mute = true;
    let h = with(settings);
    h.shortcut(Action::PushToTalk, true);
    let started = Instant::now();
    wait_until("muted", || !h.fakes.media.lock().unwrap().is_empty());
    let muted_after = started.elapsed();
    assert!(muted_after < Duration::from_millis(300), "{muted_after:?}");
    std::thread::sleep(Duration::from_millis(500));
    h.shortcut(Action::PushToTalk, false);
    wait_until("inserted", || !h.fakes.inserted().is_empty());
    assert_eq!(*h.fakes.media.lock().unwrap(), ["mute", "unmute"]);
}

#[test]
fn pausing_resumes_when_the_dictation_is_cancelled_too() {
    let mut settings = Settings::default();
    settings.media.pause = true;
    let h = with(settings);
    h.shortcut(Action::PushToTalk, true);
    std::thread::sleep(Duration::from_millis(400));
    h.shortcut(Action::Cancel, true);
    h.shortcut(Action::Cancel, false);
    h.shortcut(Action::PushToTalk, false);
    wait_until("resumed", || h.fakes.media.lock().unwrap().len() == 2);
    assert_eq!(*h.fakes.media.lock().unwrap(), ["pause", "resume"]);
}

#[test]
fn muting_and_pausing_exclude_each_other() {
    let store = SettingsStore::in_memory(Settings::default());
    let both = serde_json::json!({"media": {"mute": true, "pause": true}});
    assert!(store.update(&both).is_err());
}
