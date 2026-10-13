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
use shrimp_lib::edit::{CleanupLevel, EditTimeout, Editor, FixedLibrary, Snapshot};
use shrimp_lib::hotkeys::{Action, Binding, ShortcutEvent, macos_defaults};
use shrimp_lib::llm::{LlmError, LlmProvider, LlmProviderId, LlmRequest, LlmResponse, LlmSource};
use shrimp_lib::notify::Notice;
use shrimp_lib::platform::{
    AppIdentity, CuePlayer, FocusInspector, HotkeyBackend, InsertOutcome, InsertText, Permission,
    PermissionStatus, Permissions, PlatformError, TextInserter,
};
use shrimp_lib::settings::{Settings, SettingsStore};
use shrimp_lib::sounds::Cue;
use shrimp_lib::storage::{DictionaryEntry, EntrySource, Snippet};
use shrimp_lib::stt::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};

const TEXT: &str = "Hello from Shrimp.";
const WAIT: Duration = Duration::from_secs(5);

#[derive(Default)]
struct Fakes {
    bindings: Mutex<Vec<Vec<Action>>>,
    inserted: Mutex<Vec<String>>,
    inserted_html: Mutex<Vec<Option<String>>>,
    copied: Mutex<Vec<String>>,
    vocabulary: Mutex<Vec<Vec<String>>>,
    prompts: Mutex<Vec<LlmRequest>>,
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
    fn insert(&self, text: &InsertText, _: Duration) -> Result<InsertOutcome, PlatformError> {
        self.0.inserted.lock().unwrap().push(text.plain.clone());
        self.0.inserted_html.lock().unwrap().push(text.html.clone());
        Ok(InsertOutcome::Pasted)
    }

    fn copy(&self, text: &InsertText) -> Result<(), PlatformError> {
        self.0.copied.lock().unwrap().push(text.plain.clone());
        Ok(())
    }
}

struct FakeFocus;

impl FocusInspector for FakeFocus {
    fn frontmost_app(&self) -> Option<AppIdentity> {
        Some(AppIdentity {
            bundle_id: "com.tinyspeck.slackmacgap".into(),
            name: "Slack".into(),
        })
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

    async fn transcribe(
        &self,
        clip: &AudioClip,
        options: &SttOptions,
    ) -> Result<Transcript, SttError> {
        self.0
            .vocabulary
            .lock()
            .unwrap()
            .push(options.vocabulary.clone());
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

/// A cloud engine that always fails, to exercise the Whisper fallback.
struct FailingCloud;

#[async_trait]
impl SttEngine for FailingCloud {
    fn id(&self) -> SttEngineId {
        SttEngineId::Groq
    }

    async fn transcribe(&self, _: &AudioClip, _: &SttOptions) -> Result<Transcript, SttError> {
        Err(SttError::Unavailable("Groq rate limit reached.".into()))
    }
}

/// An LLM that answers from a script and records each request.
struct FakeLlm {
    fakes: Arc<Fakes>,
    replies: Mutex<Vec<Result<String, LlmError>>>,
    delay: Duration,
}

#[async_trait]
impl LlmProvider for FakeLlm {
    fn id(&self) -> LlmProviderId {
        LlmProviderId::Anthropic
    }

    fn model(&self) -> &str {
        "fake"
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        self.fakes.prompts.lock().unwrap().push(request.clone());
        tokio::time::sleep(self.delay).await;
        let mut replies = self.replies.lock().unwrap();
        let reply = if replies.len() > 1 {
            replies.remove(0)
        } else {
            replies[0].clone()
        };
        reply.map(|text| LlmResponse { text })
    }
}

struct FakeLlmSource(Arc<FakeLlm>);

#[async_trait]
impl LlmSource for FakeLlmSource {
    async fn provider(
        &self,
        _: &shrimp_lib::settings::AiSettings,
    ) -> Result<Arc<dyn LlmProvider>, LlmError> {
        Ok(Arc::clone(&self.0) as Arc<dyn LlmProvider>)
    }
}

/// What the edit pipeline and STT look like in a test.
struct Setup {
    cleanup: CleanupLevel,
    replies: Vec<Result<String, LlmError>>,
    llm_delay: Duration,
    library: Snapshot,
    cloud_engine: bool,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            cleanup: CleanupLevel::None,
            replies: vec![Ok(String::new())],
            llm_delay: Duration::ZERO,
            library: Snapshot::default(),
            cloud_engine: false,
        }
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
        Self::with(seconds, amplitude, timing, Setup::default())
    }

    fn with(seconds: f32, amplitude: f32, timing: Timing, setup: Setup) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source.wav");
        write_tone(&path, seconds, amplitude);
        let fakes = Arc::new(Fakes::default());
        let mut settings = Settings::default();
        settings.ai.cleanup = setup.cleanup;
        let llm = Arc::new(FakeLlm {
            fakes: Arc::clone(&fakes),
            replies: Mutex::new(setup.replies),
            delay: setup.llm_delay,
        });
        let editor = Arc::new(Editor::new(
            Arc::new(SettingsStore::in_memory(settings)),
            Arc::new(FakeLlmSource(llm)),
            Arc::new(FixedLibrary(setup.library)),
            EditTimeout {
                base: Duration::from_millis(300),
                per_word: Duration::ZERO,
                max: Duration::from_millis(300),
            },
        ));
        let local: Arc<dyn SttEngine> = Arc::new(FakeEngine(Arc::clone(&fakes)));
        let (engine, fallback_engine) = if setup.cloud_engine {
            (Arc::new(FailingCloud) as Arc<dyn SttEngine>, Some(local))
        } else {
            (local, None)
        };
        let controller = dictation::spawn(Services {
            hotkeys: Box::new(FakeHotkeys(Arc::clone(&fakes))),
            bindings: macos_defaults(),
            push_to_talk_label: "fn".into(),
            inserter: Arc::new(FakeInserter(Arc::clone(&fakes))),
            focus: Arc::new(FakeFocus),
            permissions: Arc::new(Granted),
            sounds: Arc::new(FakeSounds(Arc::clone(&fakes))),
            engine,
            fallback_engine,
            languages: vec!["en".into()],
            editor,
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

fn ai(cleanup: CleanupLevel, replies: Vec<Result<String, LlmError>>) -> Setup {
    Setup {
        cleanup,
        replies,
        ..Setup::default()
    }
}

fn dictate(h: &Harness) {
    h.hold(Action::PushToTalk, Duration::from_millis(700));
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
