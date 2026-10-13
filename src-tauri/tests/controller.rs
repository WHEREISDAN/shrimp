//! The dictation controller end to end with fake platform services and a
//! WAV file standing in for the microphone.

mod support;

use support::*;

#[test]
fn hold_to_talk_records_transcribes_and_inserts() {
    let h = Harness::speech(5.0);
    h.shortcut(Action::PushToTalk, true);
    wait_until("recording", || {
        h.fakes.last_state()
            == Some(DictationState::Recording {
                mode: Mode::Hold,
                command: false,
            })
    });
    let bindings = h.fakes.bindings.lock().unwrap().clone();
    assert!(
        bindings.last().unwrap().contains(&Action::Cancel),
        "Esc bound while recording"
    );

    std::thread::sleep(Duration::from_millis(700));
    h.shortcut(Action::PushToTalk, false);
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
    wait_until("idle", || {
        h.fakes.last_state() == Some(DictationState::Idle)
    });

    assert_eq!(h.fakes.cues(), [Cue::Start, Cue::Stop]);
    let states = h.fakes.states.lock().unwrap().clone();
    assert!(states.contains(&DictationState::Processing));
    let bindings = h.fakes.bindings.lock().unwrap().clone();
    assert!(
        !bindings.last().unwrap().contains(&Action::Cancel),
        "Esc unbound after"
    );
    let samples = h.fakes.transcribed.lock().unwrap()[0];
    assert!(
        samples >= 16_000 * 6 / 10,
        "about 0.7 s recorded, got {samples} samples"
    );
}

#[test]
fn a_short_tap_inserts_nothing_and_shows_the_tip() {
    let h = Harness::speech(5.0);
    h.hold(Action::PushToTalk, Duration::from_millis(100));
    wait_until("the tip", || h.fakes.has_notice("Hold fn to dictate"));
    assert_eq!(h.fakes.last_state(), Some(DictationState::Idle));
    assert!(h.fakes.inserted().is_empty());
    assert!(h.fakes.transcribed.lock().unwrap().is_empty());
}

#[test]
fn double_tap_locks_hands_free_until_the_next_press() {
    let h = Harness::speech(5.0);
    h.hold(Action::PushToTalk, Duration::from_millis(80));
    std::thread::sleep(Duration::from_millis(100));
    h.hold(Action::PushToTalk, Duration::from_millis(80));
    std::thread::sleep(Duration::from_millis(900));
    assert_eq!(
        h.fakes.last_state(),
        Some(DictationState::Recording {
            mode: Mode::Locked,
            command: false
        })
    );
    h.hold(Action::PushToTalk, Duration::from_millis(50));
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
    assert_eq!(h.fakes.cues(), [Cue::Start, Cue::Lock, Cue::Stop]);
}

#[test]
fn escape_cancels_and_paste_last_transcribes_the_cancelled_audio() {
    let h = Harness::speech(5.0);
    h.shortcut(Action::PushToTalk, true);
    std::thread::sleep(Duration::from_millis(600));
    h.shortcut(Action::Cancel, true);
    wait_until("the cancel notice", || {
        h.fakes.has_notice("Dictation cancelled")
    });
    let bound = |h: &Harness| h.fakes.bindings.lock().unwrap().last().unwrap().clone();
    assert!(
        bound(&h).contains(&Action::Cancel),
        "Esc stays bound until released"
    );
    h.shortcut(Action::Cancel, false);
    wait_until("Esc unbound", || !bound(&h).contains(&Action::Cancel));
    h.shortcut(Action::PushToTalk, false);
    assert_eq!(h.fakes.cues(), [Cue::Start, Cue::Cancel]);
    assert!(h.fakes.inserted().is_empty());

    std::thread::sleep(Duration::from_millis(200));
    h.controller.send(ControllerInput::Menu(Action::PasteLast));
    wait_until("the retried insertion", || h.fakes.inserted() == [TEXT]);
}

#[test]
fn shortcuts_are_rejected_while_processing() {
    let h = Harness::speech(5.0);
    *h.fakes.stt_delay.lock().unwrap() = Duration::from_millis(800);
    h.hold(Action::PushToTalk, Duration::from_millis(600));
    wait_until("processing", || {
        h.fakes.last_state() == Some(DictationState::Processing)
    });
    h.hold(Action::PushToTalk, Duration::from_millis(50));
    wait_until("the busy notice", || {
        h.fakes.has_notice("Transcript currently processing")
    });
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        h.fakes.inserted(),
        [TEXT],
        "the rejected press was not queued"
    );
}

#[test]
fn the_session_limit_warns_then_stops_and_inserts() {
    let timing = Timing {
        session_warning: Duration::from_millis(400),
        session_limit: Duration::from_millis(900),
        ..fast_timing()
    };
    let h = Harness::new(5.0, 0.3, timing);
    h.hold(Action::HandsFree, Duration::from_millis(30));
    wait_until("the warning", || h.fakes.has_notice("Dictation stops in"));
    assert_eq!(h.fakes.cues(), [Cue::Start, Cue::Warning]);
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
    assert_eq!(h.fakes.cues(), [Cue::Start, Cue::Warning, Cue::Stop]);
}

#[test]
fn every_sample_from_the_first_is_captured() {
    let h = Harness::speech(1.0);
    h.hold(Action::HandsFree, Duration::from_millis(30));
    // The WAV source ends after 1 s, which finishes the dictation.
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
    let samples = h.fakes.transcribed.lock().unwrap()[0];
    assert!(
        samples >= 16_000 - 320,
        "the whole second reached STT, got {samples} samples"
    );
}

#[test]
fn silence_never_reaches_stt() {
    let h = Harness::new(1.0, 0.0, fast_timing());
    h.hold(Action::HandsFree, Duration::from_millis(30));
    wait_until("the no-speech notice", || {
        h.fakes.has_notice("No speech detected")
    });
    assert!(h.fakes.transcribed.lock().unwrap().is_empty());
    assert!(h.fakes.inserted().is_empty());
}

#[test]
fn paste_last_retries_a_failed_transcription() {
    let h = Harness::speech(5.0);
    h.fakes.failures_left.store(1, Ordering::SeqCst);
    h.hold(Action::PushToTalk, Duration::from_millis(700));
    wait_until("the model notice", || {
        h.fakes.has_notice("downloading (42%)")
    });
    assert!(h.fakes.inserted().is_empty());

    h.shortcut(Action::PasteLast, true);
    h.shortcut(Action::PasteLast, false);
    wait_until("the retried insertion", || h.fakes.inserted() == [TEXT]);

    h.controller.send(ControllerInput::Menu(Action::PasteLast));
    wait_until("a second paste", || h.fakes.inserted().len() == 2);
    assert_eq!(
        h.fakes.transcribed.lock().unwrap().len(),
        2,
        "text pasted, not re-transcribed"
    );
}

#[test]
fn copy_last_copies_the_last_transcript() {
    let h = Harness::speech(5.0);
    h.controller.send(ControllerInput::Menu(Action::CopyLast));
    h.controller.send(ControllerInput::Menu(Action::PasteLast));
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        h.fakes.copied.lock().unwrap().is_empty(),
        "nothing to copy yet"
    );
    assert!(h.fakes.inserted().is_empty(), "nothing to paste yet");
    assert_eq!(h.fakes.last_state(), None, "paste last does nothing");

    h.hold(Action::PushToTalk, Duration::from_millis(700));
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
    h.shortcut(Action::CopyLast, true);
    wait_until("the copy", || *h.fakes.copied.lock().unwrap() == [TEXT]);
}

#[test]
fn flow_bar_clicks_start_stop_and_cancel() {
    let h = Harness::speech(5.0);
    h.controller.bar(BarControl::Start);
    wait_until("hands-free recording", || {
        h.fakes.last_state()
            == Some(DictationState::Recording {
                mode: Mode::Locked,
                command: false,
            })
    });
    std::thread::sleep(Duration::from_millis(700));
    h.controller.bar(BarControl::Stop);
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);

    h.controller.bar(BarControl::Start);
    std::thread::sleep(Duration::from_millis(600));
    h.controller.bar(BarControl::Cancel);
    wait_until("idle", || h.fakes.has_notice("Dictation cancelled"));

    // A second click right after starting cancels instead of inserting nothing.
    h.controller.bar(BarControl::Start);
    std::thread::sleep(Duration::from_millis(150));
    h.controller.bar(BarControl::Start);
    wait_until("the quick cancel", || {
        h.fakes
            .cues()
            .iter()
            .filter(|cue| **cue == Cue::Cancel)
            .count()
            == 2
    });
    assert_eq!(h.fakes.inserted(), [TEXT]);
}

#[test]
fn a_missing_source_fails_with_a_notice_and_no_cue() {
    let h = Harness::speech(1.0);
    std::fs::remove_file(h._dir.path().join("source.wav")).unwrap();
    h.shortcut(Action::PushToTalk, true);
    wait_until("the error notice", || {
        h.fakes.has_notice("Couldn't start recording")
    });
    assert!(h.fakes.cues().is_empty());
    h.shortcut(Action::PushToTalk, false);
    std::thread::sleep(Duration::from_millis(100));
    assert!(h.fakes.inserted().is_empty());
    assert_ne!(h.fakes.last_state(), Some(DictationState::Processing));
}

#[test]
fn the_ai_edit_is_inserted_with_the_app_style() {
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        ai(
            CleanupLevel::Light,
            vec![Ok("Hello from Shrimp, team.".into())],
        ),
    );
    dictate(&h);
    // Slack is a work app; Formal keeps the period.
    wait_until("insertion", || {
        h.fakes.inserted() == ["Hello from Shrimp, team."]
    });
    let prompts = h.fakes.prompts.lock().unwrap().clone();
    assert_eq!(
        prompts[0].messages[0].content,
        "<transcript>Hello from Shrimp.</transcript>"
    );
    assert!(prompts[0].system.contains("Style: standard capitalization"));
    assert!(h.fakes.messages().is_empty(), "{:?}", h.fakes.messages());
}

#[test]
fn a_list_from_the_model_pastes_as_rich_text() {
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        ai(
            CleanupLevel::Light,
            vec![Ok("Hello from:\n1. Shrimp".into())],
        ),
    );
    dictate(&h);
    wait_until("insertion", || {
        h.fakes.inserted() == ["Hello from:\n1. Shrimp"]
    });
    let html = h.fakes.inserted_html.lock().unwrap()[0].clone().unwrap();
    assert!(html.contains("<ol><li>Shrimp</li></ol>"), "{html}");
}

#[test]
fn a_provider_error_inserts_the_raw_transcript_with_a_notice() {
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        ai(
            CleanupLevel::Light,
            vec![Err(LlmError::Quota(
                "You have no credits remaining.".into(),
            ))],
        ),
    );
    dictate(&h);
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
    wait_until("the notice", || {
        h.fakes.has_notice(
            "Your Anthropic account is out of credits. Inserted the transcript without AI edits.",
        )
    });
}

#[test]
fn a_slow_provider_times_out_and_the_raw_transcript_is_inserted() {
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        Setup {
            llm_delay: Duration::from_secs(3),
            ..ai(CleanupLevel::Light, vec![Ok("Too late.".into())])
        },
    );
    dictate(&h);
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
    assert!(h.fakes.has_notice("Anthropic took too long."));
}

#[test]
fn an_answer_instead_of_an_edit_is_discarded() {
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        ai(
            CleanupLevel::Light,
            vec![Ok("Sure! Here is a cheerful greeting for your team.".into())],
        ),
    );
    dictate(&h);
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
}

#[test]
fn a_failed_cloud_engine_falls_back_to_whisper() {
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        Setup {
            cloud_engine: true,
            ..Setup::default()
        },
    );
    dictate(&h);
    wait_until("insertion", || h.fakes.inserted() == [TEXT]);
    assert!(
        h.fakes
            .has_notice("Groq rate limit reached. Used Whisper on this Mac instead.")
    );
}

fn entry(id: i64, term: &str, misspelling: Option<&str>) -> DictionaryEntry {
    DictionaryEntry {
        id,
        term: term.into(),
        misspelling: misspelling.map(Into::into),
        source: EntrySource::Manual,
        pinned: false,
        uses: 0,
        created_at: 0,
    }
}

#[test]
fn dictionary_terms_reach_stt_and_corrections_apply() {
    let library = Snapshot {
        dictionary: vec![entry(1, "Tauri", None), entry(2, "SHRIMP", Some("shrimp"))],
        snippets: Vec::new(),
    };
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        Setup {
            library,
            ..Setup::default()
        },
    );
    dictate(&h);
    wait_until("insertion", || h.fakes.inserted() == ["Hello from SHRIMP."]);
    assert_eq!(h.fakes.vocabulary.lock().unwrap()[0], ["Tauri", "SHRIMP"]);
}

#[test]
fn a_snippet_inside_a_sentence_expands() {
    let library = Snapshot {
        dictionary: Vec::new(),
        snippets: vec![Snippet {
            id: 1,
            trigger: "from shrimp".into(),
            expansion: "from **the Shrimp team**".into(),
            created_at: 0,
            updated_at: 0,
        }],
    };
    let h = Harness::with(
        5.0,
        0.3,
        fast_timing(),
        Setup {
            library,
            ..ai(CleanupLevel::Light, vec![Ok("Hello from Shrimp!".into())])
        },
    );
    dictate(&h);
    wait_until("insertion", || {
        h.fakes.inserted() == ["Hello from the Shrimp team!"]
    });
    let prompts = h.fakes.prompts.lock().unwrap().clone();
    assert!(
        prompts[0]
            .system
            .contains("word for word, in place: \"from shrimp\".")
    );
    let html = h.fakes.inserted_html.lock().unwrap()[0].clone().unwrap();
    assert!(html.contains("<b>the Shrimp team</b>"), "{html}");
}
