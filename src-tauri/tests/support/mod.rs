//! Fakes for the platform, speech and LLM services, and a harness that
//! runs the dictation controller with them and a WAV file as the microphone.
#![allow(dead_code, unused_imports)]

pub use std::path::Path;
pub use std::sync::atomic::{AtomicUsize, Ordering};
pub use std::sync::{Arc, Mutex};
pub use std::time::{Duration, Instant};

pub use async_trait::async_trait;
pub use handy_keys::Modifiers;
pub use shrimp_lib::audio::{
    AudioClip, AudioSourceSpec, DeviceKind, DeviceProvider, FakeBehaviour, FakeDevice, FakeDevices,
    InputDevice,
};
pub use shrimp_lib::dictation::gestures::BarControl;
pub use shrimp_lib::dictation::{
    self, Answer, ControllerHandle, ControllerInput, DictationState, DictationUi, Mode,
    RecordingStatus, Services, Timing, TransformAction, TransformView,
};
pub use shrimp_lib::edit::{CleanupLevel, EditTimeout, Editor, FixedLibrary, Snapshot};
pub use shrimp_lib::hotkeys::{
    Action, Binding, HotkeyEvent, RecordUpdate, Recorded, Recording, ShortcutEvent, Trigger,
    macos_defaults, macos_label,
};
pub use shrimp_lib::llm::{
    LlmError, LlmProvider, LlmProviderId, LlmRequest, LlmResponse, LlmSource,
};
pub use shrimp_lib::mic::Microphones;
pub use shrimp_lib::notify::Notice;
pub use shrimp_lib::platform::{
    AppIdentity, ContextRequest, CuePlayer, FieldContext, FocusInspector, HotkeyBackend,
    InsertOptions, InsertOutcome, InsertText, Launcher, MediaControl, Permission, PermissionStatus,
    Permissions, PlatformError, Restore, SyntheticKey, TextInserter, TextRange,
};
pub use shrimp_lib::settings::{Settings, SettingsStore};
pub use shrimp_lib::sounds::Cue;
pub use shrimp_lib::sounds::{SettingsCues, SoundingNotices};
pub use shrimp_lib::storage::{DictionaryEntry, EntrySource, Snippet};
pub use shrimp_lib::stt::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};

pub const TEXT: &str = "Hello from Shrimp.";
pub const WAIT: Duration = Duration::from_secs(5);

#[derive(Default)]
pub struct Fakes {
    pub bindings: Mutex<Vec<Vec<Action>>>,
    /// The last bindings set, with their triggers.
    pub bound: Mutex<Vec<Binding>>,
    pub protected: Mutex<Vec<Action>>,
    /// Whether a shortcut is being recorded through the hooks.
    pub recording_shortcut: Mutex<bool>,
    pub recording_statuses: Mutex<Vec<RecordingStatus>>,
    pub inserted: Mutex<Vec<String>>,
    pub inserted_html: Mutex<Vec<Option<String>>>,
    pub insert_options: Mutex<Vec<InsertOptions>>,
    pub app: Mutex<Option<AppIdentity>>,
    pub field: Mutex<Option<FieldContext>>,
    pub field_delay: Mutex<Duration>,
    /// What reads after the first return, when set (the field changed).
    pub field_later: Mutex<Option<FieldContext>>,
    pub field_reads: AtomicUsize,
    /// What each read returns in turn (the last one repeats), when set.
    pub field_sequence: Mutex<Vec<FieldContext>>,
    /// What the fake engine transcribes; TEXT when None.
    pub transcript: Mutex<Option<String>>,
    pub pressed: Mutex<Vec<(SyntheticKey, Modifiers)>>,
    /// What copying the selection returns (Command Mode's fallback).
    pub selection_copy: Mutex<Option<String>>,
    pub opened: Mutex<Vec<String>>,
    pub answers: Mutex<Vec<Answer>>,
    pub changes: Mutex<Vec<TransformView>>,
    /// Ranges selected through the focus inspector.
    pub selected_ranges: Mutex<Vec<TextRange>>,
    pub copied: Mutex<Vec<String>>,
    pub vocabulary: Mutex<Vec<Vec<String>>>,
    pub prompts: Mutex<Vec<LlmRequest>>,
    pub cues: Mutex<Vec<Cue>>,
    pub states: Mutex<Vec<DictationState>>,
    pub notices: Mutex<Vec<Notice>>,
    pub dismissed: Mutex<Vec<u64>>,
    /// "mute", "unmute", "pause", "resume", in order.
    pub media: Mutex<Vec<&'static str>>,
    pub transcribed: Mutex<Vec<usize>>,
    pub failures_left: AtomicUsize,
    pub stt_delay: Mutex<Duration>,
    /// The languages each transcription was asked for.
    pub languages: Mutex<Vec<Vec<String>>>,
    /// What the fake engine transcribes in, and what a language check finds.
    pub spoken: Mutex<Option<String>>,
    pub detected: Mutex<Option<(String, f32)>>,
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
    fn start(&mut self, _: Box<dyn Fn(HotkeyEvent) + Send + Sync>) -> Result<(), PlatformError> {
        Ok(())
    }

    fn set_bindings(&mut self, bindings: &[Binding]) {
        let actions = bindings.iter().map(|binding| binding.action).collect();
        self.0.bindings.lock().unwrap().push(actions);
        *self.0.bound.lock().unwrap() = bindings.to_vec();
    }

    fn set_protected(&mut self, actions: &[Action]) {
        *self.0.protected.lock().unwrap() = actions.to_vec();
    }

    fn set_recording(&mut self, recording: Option<Recording>) {
        *self.0.recording_shortcut.lock().unwrap() = recording.is_some();
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

    fn press(&self, key: SyntheticKey, extra: Modifiers) -> Result<(), PlatformError> {
        self.0.pressed.lock().unwrap().push((key, extra));
        Ok(())
    }

    fn copy_selection(&self) -> Result<Option<String>, PlatformError> {
        Ok(self.0.selection_copy.lock().unwrap().clone())
    }
}

pub struct FakeLauncher(pub Arc<Fakes>);

impl Launcher for FakeLauncher {
    fn open_url(&self, url: &str) -> Result<(), PlatformError> {
        self.0.opened.lock().unwrap().push(url.to_owned());
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
        let reads = self.0.field_reads.fetch_add(1, Ordering::SeqCst);
        let sequence = self.0.field_sequence.lock().unwrap();
        if let Some(last) = sequence.last() {
            return Some(sequence.get(reads).unwrap_or(last).clone());
        }
        drop(sequence);
        match &*self.0.field_later.lock().unwrap() {
            Some(later) if reads > 0 => Some(later.clone()),
            _ => self.0.field.lock().unwrap().clone(),
        }
    }

    fn select_range(&self, field_id: u64, range: TextRange) -> bool {
        let field = self.0.field.lock().unwrap();
        let same = field
            .as_ref()
            .is_some_and(|field| field.field_id == field_id);
        if same {
            self.0.selected_ranges.lock().unwrap().push(range);
        }
        same
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

/// Something is always playing.
pub struct FakeMedia(pub Arc<Fakes>);

impl MediaControl for FakeMedia {
    fn mute(&self) -> Option<Restore> {
        self.0.media.lock().unwrap().push("mute");
        let fakes = Arc::clone(&self.0);
        Some(Box::new(move || fakes.media.lock().unwrap().push("unmute")))
    }

    fn pause(&self) -> Option<Restore> {
        self.0.media.lock().unwrap().push("pause");
        let fakes = Arc::clone(&self.0);
        Some(Box::new(move || fakes.media.lock().unwrap().push("resume")))
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
        self.0
            .languages
            .lock()
            .unwrap()
            .push(options.languages.clone());
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
            language: Some(self.0.spoken.lock().unwrap().clone().unwrap_or("en".into())),
        })
    }

    async fn detect_language(&self, _: &AudioClip) -> Option<(String, f32)> {
        self.0.detected.lock().unwrap().clone()
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
    /// Record from these fake microphones through the microphone chooser
    /// instead of a WAV file.
    pub microphones: Option<Arc<FakeDevices>>,
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
            microphones: None,
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

    fn dismiss(&self, id: u64) {
        self.0.dismissed.lock().unwrap().push(id);
    }

    fn answer(&self, answer: Answer) {
        self.0.answers.lock().unwrap().push(answer);
    }

    fn changes(&self, view: TransformView) {
        self.0.changes.lock().unwrap().push(view);
    }

    fn shortcut_recording(&self, status: RecordingStatus) {
        self.0.recording_statuses.lock().unwrap().push(status);
    }
}

pub struct Harness {
    pub controller: ControllerHandle,
    pub fakes: Arc<Fakes>,
    pub microphones: Option<Arc<Microphones>>,
    pub settings: Arc<SettingsStore>,
    pub library: Arc<FixedLibrary>,
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
        let settings = Arc::new(SettingsStore::in_memory(settings));
        let library = Arc::new(FixedLibrary::new(setup.library));
        let editor = Arc::new(Editor::new(
            Arc::clone(&settings),
            Arc::new(FakeLlmSource(llm)),
            Arc::clone(&library) as Arc<dyn shrimp_lib::edit::Library>,
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
        let cues: Arc<dyn CuePlayer> = Arc::new(SettingsCues::new(
            Arc::new(FakeSounds(Arc::clone(&fakes))),
            Arc::clone(&settings),
        ));
        let ui: Arc<dyn DictationUi> = Arc::new(SoundingNotices::new(
            Arc::new(FakeUi(Arc::clone(&fakes))),
            Arc::clone(&cues),
        ));
        let microphones = setup.microphones.map(|devices| {
            Microphones::new(
                Arc::new(devices) as Arc<dyn DeviceProvider>,
                Arc::clone(&settings),
                Arc::clone(&ui),
                Box::new(|_| {}),
            )
        });
        let source = if microphones.is_some() {
            AudioSourceSpec::Microphone { device: None }
        } else {
            AudioSourceSpec::WavFile(path)
        };
        let controller = dictation::spawn(Services {
            hotkeys: Box::new(FakeHotkeys(Arc::clone(&fakes))),
            bindings: macos_defaults(true),
            label: macos_label,
            inserter: Arc::new(FakeInserter(Arc::clone(&fakes))),
            focus: Arc::new(FakeFocus(Arc::clone(&fakes))),
            permissions: Arc::new(Granted),
            sounds: cues,
            launcher: Arc::new(FakeLauncher(Arc::clone(&fakes))),
            media: Arc::new(FakeMedia(Arc::clone(&fakes))),
            settings: Arc::clone(&settings),
            engine,
            fallback_engine,
            editor,
            ui,
            source,
            microphones: microphones.clone(),
            timing,
        });
        Self {
            controller,
            fakes,
            microphones,
            settings,
            library,
            _dir: dir,
        }
    }

    pub fn speech(seconds: f32) -> Self {
        Self::new(seconds, 0.3, fast_timing())
    }

    pub fn shortcut(&self, action: Action, pressed: bool) {
        self.controller
            .send(ControllerInput::Shortcut(ShortcutEvent::new(
                action, pressed,
            )));
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
        learn: shrimp_lib::dictation::learn::LearnTiming {
            settle: Duration::from_millis(50),
            poll: Duration::from_millis(100),
            watch_for: Duration::from_secs(3),
        },
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
