//! The dictation actor: one task owns all session state and reacts to
//! shortcuts, Flow Bar clicks, timers and pipeline results on one channel.

mod language;
mod microphone;
mod recording;
mod shortcuts;
mod transforms;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::time::MissedTickBehavior;

use super::command::CommandRunner;
use super::gestures::{BarControl, CancelReason, Command, Gestures, Input, RecordingMode};
use super::learn::{self, LearnTiming};
use super::media::{MediaSession, QUIET_AFTER};
use super::outcomes;
use super::pipeline::Pipeline;
use super::state::{
    ControllerInput, DictationState, DictationUi, LastDictation, LastStatus, Outcome, Processed,
    Timing, TransformAction,
};
use super::transform::TransformRun;
use crate::audio::{AudioError, AudioSourceSpec, Recorder, level};
use crate::context::{self, DictationContext, PendingContext};
use crate::edit::Editor;
use crate::hotkeys::{Action, Binding, HotkeyEvent, Trigger};
use crate::mic::Microphones;
use crate::notify::{Notice, NoticeAction};
use crate::platform::{
    CuePlayer, FocusInspector, HotkeyBackend, Launcher, MediaControl, Permission, PermissionStatus,
    Permissions, PlatformError, TextInserter,
};
use crate::settings::SettingsStore;
use crate::sounds::Cue;
use crate::stt::SttEngine;

/// Everything the controller needs, injected so tests can use fakes.
pub struct Services {
    pub hotkeys: Box<dyn HotkeyBackend>,
    /// The platform's default bindings.
    pub bindings: Vec<Binding>,
    /// Formats a shortcut for tips and the Shortcuts dialog.
    pub label: fn(&Trigger) -> String,
    pub inserter: Arc<dyn TextInserter>,
    pub focus: Arc<dyn FocusInspector>,
    pub permissions: Arc<dyn Permissions>,
    pub sounds: Arc<dyn CuePlayer>,
    pub launcher: Arc<dyn Launcher>,
    /// Mutes or pauses playing audio while dictating (SND-03, SND-04).
    pub media: Arc<dyn MediaControl>,
    pub settings: Arc<SettingsStore>,
    /// The engine the settings choose.
    pub engine: Arc<dyn SttEngine>,
    /// Local Whisper, for when a cloud engine fails.
    pub fallback_engine: Option<Arc<dyn SttEngine>>,
    pub editor: Arc<Editor>,
    pub ui: Arc<dyn DictationUi>,
    pub source: AudioSourceSpec,
    /// Chooses the microphone when `source` is the default microphone.
    pub microphones: Option<Arc<Microphones>>,
    pub timing: Timing,
}

/// Cheap to clone; the app keeps one in managed state.
#[derive(Clone)]
pub struct ControllerHandle {
    tx: UnboundedSender<ControllerInput>,
    state: Arc<Mutex<DictationState>>,
    source: Arc<Mutex<AudioSourceSpec>>,
    secure_input: Arc<Mutex<Option<String>>>,
}

impl ControllerHandle {
    pub fn send(&self, input: ControllerInput) {
        if self.tx.send(input).is_err() {
            log::warn!("dictation controller has stopped");
        }
    }

    pub fn bar(&self, control: BarControl) {
        self.send(ControllerInput::Bar(control));
    }

    pub fn state(&self) -> DictationState {
        *lock(&self.state)
    }

    pub fn audio_source(&self) -> AudioSourceSpec {
        lock(&self.source).clone()
    }

    /// Takes effect from the next dictation.
    pub fn set_audio_source(&self, spec: AudioSourceSpec) {
        log::info!("audio source set to {spec}");
        *lock(&self.source) = spec;
    }

    /// The app that has Secure Keyboard Entry on, if any (KEYS-09).
    pub fn secure_input(&self) -> Option<String> {
        lock(&self.secure_input).clone()
    }
}

pub fn spawn(services: Services) -> ControllerHandle {
    let (tx, rx) = unbounded_channel();
    let state = Arc::new(Mutex::new(DictationState::Idle));
    let source = Arc::new(Mutex::new(services.source.clone()));
    let secure_input = Arc::new(Mutex::new(None));
    let mut controller = Controller::new(
        services,
        tx.clone(),
        Arc::clone(&state),
        Arc::clone(&source),
    );
    controller.secure_input = Arc::clone(&secure_input);
    tauri::async_runtime::spawn(controller.run(rx));
    ControllerHandle {
        tx,
        state,
        source,
        secure_input,
    }
}

struct Recording {
    recorder: Recorder,
    session: u64,
    started: Instant,
    /// The microphone and its warnings, when recording from one (MIC-07).
    microphone: Option<microphone::MicSession>,
    /// Muted or paused audio to restore when recording ends.
    media: Option<MediaSession>,
    /// The app the user dictates into and its focused field, read while
    /// recording (CTX-01).
    context: PendingContext,
}

struct Controller {
    hotkeys: Box<dyn HotkeyBackend>,
    bindings: Vec<Binding>,
    label: fn(&Trigger) -> String,
    inserter: Arc<dyn TextInserter>,
    focus: Arc<dyn FocusInspector>,
    permissions: Arc<dyn Permissions>,
    sounds: Arc<dyn CuePlayer>,
    launcher: Arc<dyn Launcher>,
    media: Arc<dyn MediaControl>,
    settings: Arc<SettingsStore>,
    ui: Arc<dyn DictationUi>,
    microphones: Option<Arc<Microphones>>,
    pipeline: Pipeline,
    timing: Timing,
    tx: UnboundedSender<ControllerInput>,
    state: Arc<Mutex<DictationState>>,
    source: Arc<Mutex<AudioSourceSpec>>,
    gestures: Gestures,
    recording: Option<Recording>,
    sessions: u64,
    last: Option<LastDictation>,
    hotkeys_started: bool,
    accessibility_notice_shown: bool,
    paste_delay: Duration,
    /// Esc is down; it stays bound until released.
    cancel_held: bool,
    /// When the last dictation was pasted, for transforms (TRF-01).
    last_inserted_at: Option<Instant>,
    /// The last transform, for its changes view (TRF-04).
    last_transform: Option<TransformRun>,
    /// A retry is running; its result updates the changes view.
    retrying: bool,
    /// Moves on with every recording, which stops the auto-learn watcher.
    generation: Arc<AtomicU64>,
    learn_timing: LearnTiming,
    /// The action and binding position of a shortcut being recorded.
    shortcut_target: Option<(Action, Option<usize>)>,
    /// The language the different-language card offers to add (LANG-04).
    detected_language: Option<String>,
    secure_input: Arc<Mutex<Option<String>>>,
}

impl Controller {
    fn new(
        services: Services,
        tx: UnboundedSender<ControllerInput>,
        state: Arc<Mutex<DictationState>>,
        source: Arc<Mutex<AudioSourceSpec>>,
    ) -> Self {
        let pipeline = Pipeline {
            engine: services.engine,
            fallback: services.fallback_engine,
            editor: services.editor,
            inserter: Arc::clone(&services.inserter),
            settings: Arc::clone(&services.settings),
            clipboard_hold: services.timing.clipboard_hold,
            remote_clipboard_hold: services.timing.remote_clipboard_hold,
        };
        Self {
            hotkeys: services.hotkeys,
            bindings: services.bindings,
            label: services.label,
            inserter: services.inserter,
            focus: services.focus,
            permissions: services.permissions,
            sounds: services.sounds,
            launcher: services.launcher,
            media: services.media,
            settings: services.settings,
            ui: services.ui,
            microphones: services.microphones,
            pipeline,
            timing: services.timing,
            tx,
            state,
            source,
            gestures: Gestures::default(),
            recording: None,
            sessions: 0,
            last: None,
            hotkeys_started: false,
            accessibility_notice_shown: false,
            paste_delay: Duration::ZERO,
            cancel_held: false,
            last_inserted_at: None,
            last_transform: None,
            retrying: false,
            generation: Arc::new(AtomicU64::new(0)),
            learn_timing: services.timing.learn,
            shortcut_target: None,
            detected_language: None,
            secure_input: Arc::new(Mutex::new(None)),
        }
    }

    async fn run(mut self, mut rx: UnboundedReceiver<ControllerInput>) {
        self.start_hotkeys();
        let mut ticker = tokio::time::interval(self.timing.level_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                input = rx.recv() => match input {
                    Some(input) => self.handle(input),
                    None => break,
                },
                _ = ticker.tick(), if self.recording.is_some() => self.poll_audio(),
            }
        }
        self.hotkeys.stop();
    }

    fn handle(&mut self, input: ControllerInput) {
        match input {
            ControllerInput::Shortcut(event) if event.pressed => {
                self.cancel_held |= event.action == Action::Cancel;
                match event.action {
                    Action::PressEnter => self.press_enter(event.extra),
                    Action::CommandMode if !self.settings.get().experimental.command_mode => {
                        self.command_mode_off();
                    }
                    Action::Polish | Action::PromptEngineer | Action::CustomTransform(_) => {
                        self.start_transform(event.action);
                    }
                    Action::ViewChanges => self.transform_action(TransformAction::View),
                    action => self.gesture(Input::Press(action)),
                }
            }
            ControllerInput::Shortcut(event) => {
                if event.action == Action::Cancel {
                    self.cancel_held = false;
                    if self.recording.is_none() {
                        self.refresh_bindings();
                    }
                }
                if is_gesture(event.action) {
                    self.gesture(Input::Release(event.action));
                }
            }
            ControllerInput::OtherKey => self.interrupted(),
            ControllerInput::RecordShortcut { action, index } => {
                self.record_shortcut(action, index);
            }
            ControllerInput::CancelRecording => self.cancel_recording(),
            ControllerInput::Recording(update) => self.recording_update(update),
            ControllerInput::SecureInput(owner) => self.secure_input_changed(owner),
            ControllerInput::UndoCancel => self.undo_cancel(),
            ControllerInput::LanguageDetected { code, probability } => {
                self.language_detected(code, probability);
            }
            ControllerInput::AddDetectedLanguage => self.add_detected_language(),
            ControllerInput::SettingsChanged => self.refresh_bindings(),
            ControllerInput::Transform(action) => self.transform_action(action),
            ControllerInput::Bar(control) => self.gesture(Input::Bar(control)),
            ControllerInput::Menu(action) => {
                self.paste_delay = self.timing.menu_paste_delay;
                self.gesture(Input::Press(action));
                self.gesture(Input::Release(action));
                self.paste_delay = Duration::ZERO;
            }
            ControllerInput::TapTimeout(token) => self.gesture(Input::TapTimeout(token)),
            ControllerInput::SessionWarning(session) if self.is_current(session) => {
                self.sounds.play(Cue::Warning);
                let left = self.timing.session_limit - self.timing.session_warning;
                self.ui.notice(
                    Notice::warning(format!("Dictation stops in {}.", minutes(left)))
                        .without_sound(),
                );
            }
            ControllerInput::SessionLimit(session) if self.is_current(session) => {
                if self.gestures.force_finish() {
                    self.finish();
                }
            }
            ControllerInput::SessionWarning(_) | ControllerInput::SessionLimit(_) => {}
            ControllerInput::RetryHotkeys => self.start_hotkeys(),
            ControllerInput::Cancelled(clip) => {
                self.last = Some(LastDictation {
                    status: LastStatus::Cancelled,
                    text: None,
                    clip: Some(clip),
                });
            }
            ControllerInput::Processed(outcome) => self.processed(outcome),
        }
    }

    fn gesture(&mut self, input: Input) {
        if let Some(command) = self.gestures.handle(input, Instant::now()) {
            self.apply(command);
        }
    }

    fn apply(&mut self, command: Command) {
        match command {
            Command::Start(mode) => self.start_recording(mode),
            Command::Lock => {
                self.sounds.play(Cue::Lock);
                self.set_state(DictationState::Recording {
                    mode: RecordingMode::Locked.into(),
                    command: self.gestures.is_command(),
                });
            }
            Command::Upgrade => self.upgrade(),
            Command::Finish => self.finish(),
            Command::Cancel(reason) => self.cancel(reason),
            Command::ArmTapTimeout { token, after } => {
                self.send_after(after, ControllerInput::TapTimeout(token));
            }
            Command::RejectBusy => self
                .ui
                .notice(Notice::warning("Transcript currently processing")),
            Command::PasteLast => self.paste_last(),
            Command::CopyLast => self.copy_last(),
        }
    }

    fn start_recording(&mut self, mode: RecordingMode) {
        let (source, device) = match self.open_source() {
            Ok(Some(opened)) => opened,
            Ok(None) => {
                self.gestures.reset();
                return;
            }
            Err(error) => {
                self.recording_failed_to_start(error);
                return;
            }
        };
        // Capture starts before anything else, so the first word is kept (ENG-04).
        match Recorder::start(source) {
            Ok(recorder) => {
                self.generation.fetch_add(1, Ordering::SeqCst);
                self.sessions += 1;
                let session = self.sessions;
                let app = self.focus.frontmost_app();
                let command = self.gestures.is_command();
                if let Some(app) = &app {
                    log::info!(
                        "recording {} for {}",
                        if command { "a command" } else { "a dictation" },
                        app.bundle_id
                    );
                }
                let focus = Arc::clone(&self.focus);
                let context = if command {
                    context::capture_for_command(focus, app)
                } else {
                    context::capture(focus, app)
                };
                if let (Some(microphones), Some(device)) = (&self.microphones, &device) {
                    microphones.recording_started(device);
                }
                self.sounds.play(Cue::Start);
                let media =
                    MediaSession::begin(&self.media, self.settings.get().media, QUIET_AFTER);
                self.recording = Some(Recording {
                    recorder,
                    session,
                    started: Instant::now(),
                    microphone: device.map(microphone::MicSession::new),
                    media,
                    context,
                });
                let editor = Arc::clone(&self.pipeline.editor);
                tauri::async_runtime::spawn(async move { editor.warm_up().await });
                self.refresh_bindings();
                self.send_after(
                    self.timing.session_warning,
                    ControllerInput::SessionWarning(session),
                );
                self.send_after(
                    self.timing.session_limit,
                    ControllerInput::SessionLimit(session),
                );
                self.set_state(DictationState::Recording {
                    mode: mode.into(),
                    command,
                });
            }
            Err(error) => self.recording_failed_to_start(error),
        }
    }

    fn recording_failed_to_start(&mut self, error: AudioError) {
        log::warn!("could not start recording: {error}");
        self.gestures.reset();
        self.ui.notice(match error {
            AudioError::NoInputDevice => {
                Notice::error("No microphone detected").with_action(NoticeAction::OpenSoundSettings)
            }
            error => Notice::error(format!("Couldn't start recording: {error}")),
        });
    }

    fn microphone_allowed(&self) -> bool {
        match self.permissions.status(Permission::Microphone) {
            PermissionStatus::Granted => true,
            PermissionStatus::NotDetermined => {
                if let Err(error) = self.permissions.request(Permission::Microphone) {
                    log::warn!("microphone request failed: {error}");
                }
                self.ui.notice(Notice::info(
                    "Allow Shrimp to use the microphone, then dictate again.",
                ));
                false
            }
            PermissionStatus::Denied => {
                self.ui.notice(
                    Notice::error("Shrimp can't use the microphone.")
                        .with_action(NoticeAction::OpenMicrophoneSettings),
                );
                false
            }
        }
    }

    /// Ctrl joined a held Fn: the dictation becomes a command, which reads
    /// the selection instead of the text around the cursor.
    fn upgrade(&mut self) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        let app = recording.context.app().cloned();
        recording.context = context::capture_for_command(Arc::clone(&self.focus), app);
        let mode = self.gestures.mode().unwrap_or(RecordingMode::Hold);
        self.set_state(DictationState::Recording {
            mode: mode.into(),
            command: true,
        });
        self.refresh_bindings();
    }

    fn finish(&mut self) {
        self.release_devices();
        let Some(recording) = self.recording.take() else {
            return;
        };
        self.refresh_bindings();
        self.sounds.play(Cue::Stop);
        self.set_state(DictationState::Processing);
        let pipeline = self.pipeline.clone();
        let runner = self.gestures.is_command().then(|| CommandRunner {
            pipeline: self.pipeline.clone(),
            focus: Arc::clone(&self.focus),
            launcher: Arc::clone(&self.launcher),
        });
        let tx = self.tx.clone();
        tauri::async_runtime::spawn(async move {
            let context = recording.context;
            let clip = tokio::task::spawn_blocking(move || recording.recorder.finish()).await;
            let processed = match clip {
                Ok(Ok(clip)) => match &runner {
                    Some(runner) => runner.run(clip, context).await,
                    None => {
                        pipeline
                            .transcribe_and_insert(clip, context, Duration::ZERO)
                            .await
                    }
                },
                Ok(Err(error)) => Outcome::RecordingFailed(error.to_string()).into(),
                Err(error) => Outcome::RecordingFailed(error.to_string()).into(),
            };
            let _ = tx.send(ControllerInput::Processed(processed));
        });
    }

    fn cancel(&mut self, reason: CancelReason) {
        self.release_devices();
        let Some(recording) = self.recording.take() else {
            return;
        };
        self.refresh_bindings();
        self.set_state(DictationState::Idle);
        let command = self.gestures.is_command();
        match reason {
            CancelReason::User => {
                self.sounds.play(Cue::Cancel);
                self.ui.notice(if command {
                    Notice::info("Command cancelled")
                } else {
                    Notice::info("Dictation cancelled").with_action(NoticeAction::UndoCancel)
                });
                let tx = self.tx.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    if let Ok(clip) = recording.recorder.finish() {
                        let _ = tx.send(ControllerInput::Cancelled(clip));
                    }
                });
            }
            CancelReason::Interrupted => {
                drop(recording);
                self.sounds.play(Cue::Cancel);
                self.ui.notice(Notice::info(
                    "Dictation cancelled because another shortcut was pressed.",
                ));
            }
            CancelReason::ShortTap if command => {
                drop(recording);
                self.ui.notice(Notice::info(format!(
                    "Hold {} while you speak a command, or double-press it for hands-free.",
                    self.label_of(Action::CommandMode)
                )));
            }
            CancelReason::ShortTap => {
                drop(recording);
                self.ui.notice(Notice::info(format!(
                    "Hold {} to dictate, or double-tap it for hands-free.",
                    self.label_of(Action::PushToTalk)
                )));
            }
        }
    }

    fn paste_last(&mut self) {
        let Some(last) = self.last.clone() else {
            return;
        };
        let pipeline = self.pipeline.clone();
        let delay = self.paste_delay;
        let tx = self.tx.clone();
        let retry = matches!(last.status, LastStatus::Failed | LastStatus::Cancelled);
        let app = self.focus.frontmost_app();
        let options = pipeline.insert_options(&DictationContext::new(app.clone(), None));
        let context = PendingContext::without_field(app);
        let work: std::pin::Pin<Box<dyn Future<Output = Processed> + Send>> =
            match (last.clip, last.text) {
                (Some(clip), _) if retry => {
                    Box::pin(
                        async move { pipeline.transcribe_and_insert(clip, context, delay).await },
                    )
                }
                (_, Some(text)) => {
                    Box::pin(async move { pipeline.insert(text, options, delay).await.into() })
                }
                _ => return,
            };
        self.gestures.begin_busy();
        self.set_state(DictationState::Processing);
        tauri::async_runtime::spawn(async move {
            let _ = tx.send(ControllerInput::Processed(work.await));
        });
    }

    fn copy_last(&mut self) {
        let Some(text) = self.last.as_ref().and_then(|last| last.text.clone()) else {
            return;
        };
        let text = text.text;
        let inserter = Arc::clone(&self.inserter);
        let ui = Arc::clone(&self.ui);
        tauri::async_runtime::spawn_blocking(move || match inserter.copy(&text) {
            Ok(()) => ui.notice(Notice::info("Last transcript copied")),
            Err(error) => ui.notice(Notice::error(format!("Couldn't copy: {error}"))),
        });
    }

    fn processed(&mut self, processed: Processed) {
        self.gestures.reset();
        self.set_state(DictationState::Idle);
        for notice in processed.notices {
            self.ui.notice(notice);
        }
        match &processed.outcome {
            Outcome::Inserted(_) if !self.gestures.is_command() => {
                self.last_inserted_at = Some(Instant::now());
            }
            Outcome::Transformed(run) => self.transformed(run),
            Outcome::TransformUndone => self.last_transform = None,
            _ => {}
        }
        if let (Some(clip), Outcome::Inserted(_)) = (processed.language_check, &processed.outcome) {
            self.check_language(clip);
        }
        if let Some(target) = processed.learn
            && self.settings.get().dictionary.auto_learn
        {
            tauri::async_runtime::spawn(learn::watch(
                target,
                Arc::clone(&self.focus),
                Arc::clone(&self.pipeline.editor),
                Arc::clone(&self.ui),
                Arc::clone(&self.generation),
                self.learn_timing,
            ));
        }
        if let Some(last) = outcomes::report(processed.outcome, self.ui.as_ref()) {
            self.last = Some(last);
        }
    }

    fn poll_audio(&mut self) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        match recording.recorder.poll() {
            Ok(samples) => {
                let rms = (!samples.is_empty()).then(|| level::rms(samples));
                self.ui.level(level::display_level(rms.unwrap_or(0.0)));
                let open = recording.recorder.is_open();
                if let Some(microphone) = recording.microphone.as_mut() {
                    microphone.update(open, rms, self.ui.as_ref());
                }
            }
            Err(AudioError::Disconnected) if recording.microphone.is_some() => {
                // Every microphone is gone: keep what was said (MIC-07).
                self.ui.notice(Notice::error("Microphone disconnected"));
                if self.gestures.force_finish() {
                    self.finish();
                }
                return;
            }
            Err(error) => {
                log::warn!("recording failed: {error}");
                self.release_devices();
                self.recording = None;
                self.gestures.reset();
                self.refresh_bindings();
                self.sounds.play(Cue::Cancel);
                self.set_state(DictationState::Idle);
                self.ui
                    .notice(Notice::error(format!("Recording stopped: {error}")));
                return;
            }
        }
        // A WAV source ran out: finish as if the shortcut had been released.
        if recording.recorder.has_ended() && self.gestures.force_finish() {
            self.finish();
        }
    }

    fn start_hotkeys(&mut self) {
        if self.hotkeys_started {
            return;
        }
        self.refresh_bindings();
        let tx = self.tx.clone();
        let on_event = Box::new(move |event| {
            let input = match event {
                HotkeyEvent::Shortcut(event) => ControllerInput::Shortcut(event),
                HotkeyEvent::OtherKey => ControllerInput::OtherKey,
                HotkeyEvent::Recording(update) => ControllerInput::Recording(update),
                HotkeyEvent::SecureInput(owner) => ControllerInput::SecureInput(owner),
            };
            let _ = tx.send(input);
        });
        match self.hotkeys.start(on_event) {
            Ok(()) => {
                self.hotkeys_started = true;
                log::info!("dictation shortcuts are active");
                if self.accessibility_notice_shown {
                    self.ui.notice(Notice::info("Shortcuts are ready"));
                }
            }
            Err(PlatformError::AccessibilityMissing) => {
                if !self.accessibility_notice_shown {
                    self.accessibility_notice_shown = true;
                    self.ui.notice(
                        Notice::error("Shrimp needs Accessibility permission for its shortcuts.")
                            .with_action(NoticeAction::OpenAccessibilitySettings),
                    );
                }
                self.send_after(self.timing.hotkey_retry, ControllerInput::RetryHotkeys);
            }
            Err(error) => {
                log::error!("shortcuts unavailable: {error}");
                self.ui
                    .notice(Notice::error(format!("Shortcuts unavailable: {error}")));
            }
        }
    }

    fn set_state(&self, state: DictationState) {
        *lock(&self.state) = state;
        self.ui.state(state);
    }

    fn is_current(&self, session: u64) -> bool {
        self.recording
            .as_ref()
            .is_some_and(|recording| recording.session == session)
    }

    fn send_after(&self, delay: Duration, input: ControllerInput) {
        let tx = self.tx.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = tx.send(input);
        });
    }
}

/// Shortcuts whose release the gestures need.
fn is_gesture(action: Action) -> bool {
    matches!(
        action,
        Action::PushToTalk
            | Action::HandsFree
            | Action::Cancel
            | Action::PasteLast
            | Action::CopyLast
            | Action::CommandMode
    )
}

fn minutes(duration: Duration) -> String {
    match duration.as_secs() {
        60 => "1 minute".into(),
        seconds if seconds >= 60 && seconds % 60 == 0 => format!("{} minutes", seconds / 60),
        seconds => format!("{seconds} seconds"),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}
