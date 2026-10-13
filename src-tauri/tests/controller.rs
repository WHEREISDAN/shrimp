//! The dictation controller end to end with fake platform services and a
//! WAV file standing in for the microphone.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use shrimp_lib::audio::{AudioClip, AudioSourceSpec};
use shrimp_lib::dictation::gestures::BarControl;
use shrimp_lib::dictation::{
    self, ControllerHandle, ControllerInput, DictationState, DictationUi, Mode, Services, Timing,
};
use shrimp_lib::hotkeys::{Action, Binding, ShortcutEvent, macos_defaults};
use shrimp_lib::notify::Notice;
use shrimp_lib::platform::{
    CuePlayer, HotkeyBackend, InsertOutcome, Permission, PermissionStatus, Permissions,
    PlatformError, TextInserter,
};
use shrimp_lib::sounds::Cue;
use shrimp_lib::stt::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};

const TEXT: &str = "Hello from Shrimp.";
const WAIT: Duration = Duration::from_secs(5);

#[derive(Default)]
struct Fakes {
    bindings: Mutex<Vec<Vec<Action>>>,
    inserted: Mutex<Vec<String>>,
    copied: Mutex<Vec<String>>,
    cues: Mutex<Vec<Cue>>,
    states: Mutex<Vec<DictationState>>,
    notices: Mutex<Vec<Notice>>,
    transcribed: Mutex<Vec<usize>>,
    failures_left: AtomicUsize,
    stt_delay: Mutex<Duration>,
}

impl Fakes {
    fn cues(&self) -> Vec<Cue> {
        self.cues.lock().unwrap().clone()
    }

    fn inserted(&self) -> Vec<String> {
        self.inserted.lock().unwrap().clone()
    }

    fn messages(&self) -> Vec<String> {
        let notices = self.notices.lock().unwrap();
        notices
            .iter()
            .map(|notice| notice.message.clone())
            .collect()
    }

    fn has_notice(&self, text: &str) -> bool {
        self.messages().iter().any(|message| message.contains(text))
    }

    fn last_state(&self) -> Option<DictationState> {
        self.states.lock().unwrap().last().copied()
    }
}

struct FakeHotkeys(Arc<Fakes>);

impl HotkeyBackend for FakeHotkeys {
    fn start(&mut self, _: Box<dyn Fn(ShortcutEvent) + Send + Sync>) -> Result<(), PlatformError> {
        Ok(())
    }

    fn set_bindings(&mut self, bindings: &[Binding]) {
        let actions = bindings.iter().map(|binding| binding.action).collect();
        self.0.bindings.lock().unwrap().push(actions);
    }

    fn stop(&mut self) {}
}

struct FakeInserter(Arc<Fakes>);

impl TextInserter for FakeInserter {
    fn insert(&self, text: &str, _: Duration) -> Result<InsertOutcome, PlatformError> {
        self.0.inserted.lock().unwrap().push(text.into());
        Ok(InsertOutcome::Pasted)
    }

    fn copy(&self, text: &str) -> Result<(), PlatformError> {
        self.0.copied.lock().unwrap().push(text.into());
        Ok(())
    }
}

struct Granted;

impl Permissions for Granted {
    fn status(&self, _: Permission) -> PermissionStatus {
        PermissionStatus::Granted
    }

    fn request(&self, _: Permission) -> Result<(), PlatformError> {
        Ok(())
    }

    fn open_settings(&self, _: Permission) -> Result<(), PlatformError> {
        Ok(())
    }
}

struct FakeSounds(Arc<Fakes>);

impl CuePlayer for FakeSounds {
    fn play(&self, cue: Cue) {
        self.0.cues.lock().unwrap().push(cue);
    }
}

struct FakeEngine(Arc<Fakes>);

#[async_trait]
impl SttEngine for FakeEngine {
    fn id(&self) -> SttEngineId {
        SttEngineId::LocalWhisper
    }

    async fn transcribe(&self, clip: &AudioClip, _: &SttOptions) -> Result<Transcript, SttError> {
        let delay = *self.0.stt_delay.lock().unwrap();
        tokio::time::sleep(delay).await;
        self.0
            .transcribed
            .lock()
            .unwrap()
            .push(clip.samples().len());
        let failures = &self.0.failures_left;
        if failures.load(Ordering::SeqCst) > 0 {
            failures.fetch_sub(1, Ordering::SeqCst);
            return Err(SttError::Unavailable(
                "The speech model is downloading (42%).".into(),
            ));
        }
        Ok(Transcript {
            text: TEXT.into(),
            language: Some("en".into()),
        })
    }
}

struct FakeUi(Arc<Fakes>);

impl DictationUi for FakeUi {
    fn state(&self, state: DictationState) {
        self.0.states.lock().unwrap().push(state);
    }

    fn level(&self, _: f32) {}

    fn notice(&self, notice: Notice) {
        self.0.notices.lock().unwrap().push(notice);
    }
}

struct Harness {
    controller: ControllerHandle,
    fakes: Arc<Fakes>,
    _dir: tempfile::TempDir,
}

impl Harness {
    /// `amplitude` 0 writes digital silence.
    fn new(seconds: f32, amplitude: f32, timing: Timing) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source.wav");
        write_tone(&path, seconds, amplitude);
        let fakes = Arc::new(Fakes::default());
        let controller = dictation::spawn(Services {
            hotkeys: Box::new(FakeHotkeys(Arc::clone(&fakes))),
            bindings: macos_defaults(),
            push_to_talk_label: "fn".into(),
            inserter: Arc::new(FakeInserter(Arc::clone(&fakes))),
            permissions: Arc::new(Granted),
            sounds: Arc::new(FakeSounds(Arc::clone(&fakes))),
            engine: Arc::new(FakeEngine(Arc::clone(&fakes))),
            stt_options: SttOptions::default(),
            ui: Arc::new(FakeUi(Arc::clone(&fakes))),
            source: AudioSourceSpec::WavFile(path),
            timing,
        });
        Self {
            controller,
            fakes,
            _dir: dir,
        }
    }

    fn speech(seconds: f32) -> Self {
        Self::new(seconds, 0.3, fast_timing())
    }

    fn shortcut(&self, action: Action, pressed: bool) {
        self.controller
            .send(ControllerInput::Shortcut(ShortcutEvent { action, pressed }));
    }

    fn hold(&self, action: Action, held: Duration) {
        self.shortcut(action, true);
        std::thread::sleep(held);
        self.shortcut(action, false);
    }
}

fn fast_timing() -> Timing {
    Timing {
        menu_paste_delay: Duration::ZERO,
        ..Timing::default()
    }
}

fn write_tone(path: &Path, seconds: f32, amplitude: f32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..(seconds * 16_000.0) as usize {
        let value = amplitude * (i as f32 * 0.07).sin();
        writer
            .write_sample((value * f32::from(i16::MAX)) as i16)
            .unwrap();
    }
    writer.finalize().unwrap();
}

fn wait_until(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn hold_to_talk_records_transcribes_and_inserts() {
    let h = Harness::speech(5.0);
    h.shortcut(Action::PushToTalk, true);
    wait_until("recording", || {
        h.fakes.last_state() == Some(DictationState::Recording { mode: Mode::Hold })
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
        Some(DictationState::Recording { mode: Mode::Locked })
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
        h.fakes.last_state() == Some(DictationState::Recording { mode: Mode::Locked })
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
