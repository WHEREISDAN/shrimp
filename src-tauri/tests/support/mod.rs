//! Fakes for the platform, speech and LLM services, and a harness that
//! runs the dictation controller with them and a WAV file as the microphone.
#![allow(dead_code, unused_imports)]

pub use std::path::Path;
pub use std::sync::atomic::{AtomicUsize, Ordering};
pub use std::sync::{Arc, Mutex};
pub use std::time::{Duration, Instant};

pub use async_trait::async_trait;
pub use shrimp_lib::audio::{AudioClip, AudioSourceSpec};
pub use shrimp_lib::dictation::gestures::BarControl;
pub use shrimp_lib::dictation::{
    self, ControllerHandle, ControllerInput, DictationState, DictationUi, Mode, Services, Timing,
};
pub use shrimp_lib::edit::{CleanupLevel, EditTimeout, Editor, FixedLibrary, Snapshot};
pub use shrimp_lib::hotkeys::{Action, Binding, ShortcutEvent, macos_defaults};
pub use shrimp_lib::llm::{
    LlmError, LlmProvider, LlmProviderId, LlmRequest, LlmResponse, LlmSource,
};
pub use shrimp_lib::notify::Notice;
pub use shrimp_lib::platform::{
    AppIdentity, ContextRequest, CuePlayer, FieldContext, FocusInspector, HotkeyBackend,
    InsertOptions, InsertOutcome, InsertText, Permission, PermissionStatus, Permissions,
    PlatformError, TextInserter,
};
pub use shrimp_lib::settings::{Settings, SettingsStore};
pub use shrimp_lib::sounds::Cue;
pub use shrimp_lib::storage::{DictionaryEntry, EntrySource, Snippet};
pub use shrimp_lib::stt::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};

pub const TEXT: &str = "Hello from Shrimp.";
pub const WAIT: Duration = Duration::from_secs(5);

#[derive(Default)]
pub struct Fakes {
    pub bindings: Mutex<Vec<Vec<Action>>>,
    pub inserted: Mutex<Vec<String>>,
    pub inserted_html: Mutex<Vec<Option<String>>>,
    pub insert_options: Mutex<Vec<InsertOptions>>,
    pub app: Mutex<Option<AppIdentity>>,
    pub field: Mutex<Option<FieldContext>>,
    pub field_delay: Mutex<Duration>,
    /// What the fake engine transcribes; TEXT when None.
    pub transcript: Mutex<Option<String>>,
    pub copied: Mutex<Vec<String>>,
    pub vocabulary: Mutex<Vec<Vec<String>>>,
    pub prompts: Mutex<Vec<LlmRequest>>,
    pub cues: Mutex<Vec<Cue>>,
    pub states: Mutex<Vec<DictationState>>,
    pub notices: Mutex<Vec<Notice>>,
    pub transcribed: Mutex<Vec<usize>>,
    pub failures_left: AtomicUsize,
    pub stt_delay: Mutex<Duration>,
}

impl Fakes {
    pub fn cues(&self) -> Vec<Cue> {
        self.cues.lock().unwrap().clone()
    }

    pub fn inserted(&self) -> Vec<String> {
        self.inserted.lock().unwrap().clone()
    }

    pub fn messages(&self) -> Vec<String> {
        let notices = self.notices.lock().unwrap();
        notices
            .iter()
            .map(|notice| notice.message.clone())
            .collect()
    }

    pub fn has_notice(&self, text: &str) -> bool {
        self.messages().iter().any(|message| message.contains(text))
    }

    pub fn last_state(&self) -> Option<DictationState> {
        self.states.lock().unwrap().last().copied()
    }
}

pub struct FakeHotkeys(pub Arc<Fakes>);

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

pub struct FakeInserter(pub Arc<Fakes>);

impl TextInserter for FakeInserter {
    fn insert(
        &self,
        text: &InsertText,
        options: &InsertOptions,
    ) -> Result<InsertOutcome, PlatformError> {
        self.0.insert_options.lock().unwrap().push(*options);
        self.0.inserted.lock().unwrap().push(text.plain.clone());
        self.0.inserted_html.lock().unwrap().push(text.html.clone());
        Ok(InsertOutcome::Pasted)
    }

    fn copy(&self, text: &InsertText) -> Result<(), PlatformError> {
        self.0.copied.lock().unwrap().push(text.plain.clone());
        Ok(())
    }
}

/// The frontmost app (Slack unless a test sets another) and its field.
pub struct FakeFocus(pub Arc<Fakes>);

pub fn app(bundle_id: &str, name: &str) -> AppIdentity {
    AppIdentity {
        bundle_id: bundle_id.into(),
        name: name.into(),
    }
}

impl FocusInspector for FakeFocus {
    fn frontmost_app(&self) -> Option<AppIdentity> {
        Some(
            self.0
                .app
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| app("com.tinyspeck.slackmacgap", "Slack")),
        )
    }

    fn read_field(&self, _: &ContextRequest) -> Option<FieldContext> {
        let delay = *self.0.field_delay.lock().unwrap();
        std::thread::sleep(delay);
        self.0.field.lock().unwrap().clone()
    }
}

pub struct Granted;

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

pub struct FakeSounds(pub Arc<Fakes>);

impl CuePlayer for FakeSounds {
    fn play(&self, cue: Cue) {
        self.0.cues.lock().unwrap().push(cue);
    }
}

pub struct FakeEngine(pub Arc<Fakes>);

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
            text: self
                .0
                .transcript
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(TEXT.into()),
            language: Some("en".into()),
        })
    }
}

/// A cloud engine that always fails, to exercise the Whisper fallback.
pub struct FailingCloud;

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
pub struct FakeLlm {
    pub fakes: Arc<Fakes>,
    pub replies: Mutex<Vec<Result<String, LlmError>>>,
    pub delay: Duration,
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

pub struct FakeLlmSource(pub Arc<FakeLlm>);

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
pub struct Setup {
    pub cleanup: CleanupLevel,
    pub replies: Vec<Result<String, LlmError>>,
    pub llm_delay: Duration,
    pub library: Snapshot,
    pub cloud_engine: bool,
    /// The frontmost app; Slack when None.
    pub app: Option<AppIdentity>,
    /// What reading the focused field returns.
    pub field: Option<FieldContext>,
    pub settings: Settings,
    pub transcript: Option<String>,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            cleanup: CleanupLevel::None,
            replies: vec![Ok(String::new())],
            llm_delay: Duration::ZERO,
            library: Snapshot::default(),
            cloud_engine: false,
            app: None,
            field: None,
            settings: Settings::default(),
            transcript: None,
        }
    }
}

pub struct FakeUi(pub Arc<Fakes>);

impl DictationUi for FakeUi {
    fn state(&self, state: DictationState) {
        self.0.states.lock().unwrap().push(state);
    }

    fn level(&self, _: f32) {}

    fn notice(&self, notice: Notice) {
        self.0.notices.lock().unwrap().push(notice);
    }
}

pub struct Harness {
    pub controller: ControllerHandle,
    pub fakes: Arc<Fakes>,
    pub _dir: tempfile::TempDir,
}

impl Harness {
    /// `amplitude` 0 writes digital silence.
    pub fn new(seconds: f32, amplitude: f32, timing: Timing) -> Self {
        Self::with(seconds, amplitude, timing, Setup::default())
    }

    pub fn with(seconds: f32, amplitude: f32, timing: Timing, setup: Setup) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source.wav");
        write_tone(&path, seconds, amplitude);
        let fakes = Arc::new(Fakes::default());
        *fakes.app.lock().unwrap() = setup.app;
        *fakes.field.lock().unwrap() = setup.field;
        *fakes.transcript.lock().unwrap() = setup.transcript;
        let mut settings = setup.settings;
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
            focus: Arc::new(FakeFocus(Arc::clone(&fakes))),
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

    pub fn speech(seconds: f32) -> Self {
        Self::new(seconds, 0.3, fast_timing())
    }

    pub fn shortcut(&self, action: Action, pressed: bool) {
        self.controller
            .send(ControllerInput::Shortcut(ShortcutEvent { action, pressed }));
    }

    pub fn hold(&self, action: Action, held: Duration) {
        self.shortcut(action, true);
        std::thread::sleep(held);
        self.shortcut(action, false);
    }
}

pub fn fast_timing() -> Timing {
    Timing {
        menu_paste_delay: Duration::ZERO,
        ..Timing::default()
    }
}

pub fn write_tone(path: &Path, seconds: f32, amplitude: f32) {
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

pub fn wait_until(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub fn ai(cleanup: CleanupLevel, replies: Vec<Result<String, LlmError>>) -> Setup {
    Setup {
        cleanup,
        replies,
        ..Setup::default()
    }
}

pub fn dictate(h: &Harness) {
    h.hold(Action::PushToTalk, Duration::from_millis(700));
}
