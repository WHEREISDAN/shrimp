//! The dictation actor: one task owns all session state and reacts to
//! shortcuts, Flow Bar clicks, timers and pipeline results on one channel.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::time::MissedTickBehavior;

use handy_keys::Modifiers;

use super::bindings::{self, Moment};
use super::command::CommandRunner;
use super::gestures::{BarControl, CancelReason, Command, Gestures, Input, RecordingMode};
use super::outcomes;
use super::pipeline::Pipeline;
use super::state::{
    ControllerInput, DictationState, DictationUi, LastDictation, LastStatus, Outcome, Processed,
    Timing,
};
use crate::audio::{AudioError, AudioSourceSpec, Recorder, level};
use crate::context::{self, DictationContext, PendingContext};
use crate::edit::Editor;
use crate::hotkeys::{Action, Binding};
use crate::notify::{Notice, NoticeAction};
use crate::platform::{
    CuePlayer, FocusInspector, HotkeyBackend, Launcher, Permission, PermissionStatus, Permissions,
    PlatformError, SyntheticKey, TextInserter,
};
use crate::settings::SettingsStore;
use crate::sounds::Cue;
use crate::stt::SttEngine;

/// "Command mode is toggled off" shows at most this often (CMD-06).
const COMMAND_OFF_NOTICES: u32 = 3;
const SECONDS_PER_DAY: u64 = 86_400;

/// Everything the controller needs, injected so tests can use fakes.
pub struct Services {
    pub hotkeys: Box<dyn HotkeyBackend>,
    pub bindings: Vec<Binding>,
    /// Display names of the push-to-talk and Command Mode shortcuts, for tips.
    pub push_to_talk_label: String,
    pub command_mode_label: String,
    pub inserter: Arc<dyn TextInserter>,
    pub focus: Arc<dyn FocusInspector>,
    pub permissions: Arc<dyn Permissions>,
    pub sounds: Arc<dyn CuePlayer>,
    pub launcher: Arc<dyn Launcher>,
    pub settings: Arc<SettingsStore>,
    /// The engine the settings choose.
    pub engine: Arc<dyn SttEngine>,
    /// Local Whisper, for when a cloud engine fails.
    pub fallback_engine: Option<Arc<dyn SttEngine>>,
    /// Languages the user speaks (ISO 639-1); empty means auto-detect.
    pub languages: Vec<String>,
    pub editor: Arc<Editor>,
    pub ui: Arc<dyn DictationUi>,
    pub source: AudioSourceSpec,
    pub timing: Timing,
}

/// Cheap to clone; the app keeps one in managed state.
#[derive(Clone)]
pub struct ControllerHandle {
    tx: UnboundedSender<ControllerInput>,
    state: Arc<Mutex<DictationState>>,
    source: Arc<Mutex<AudioSourceSpec>>,
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
}

pub fn spawn(services: Services) -> ControllerHandle {
    let (tx, rx) = unbounded_channel();
    let state = Arc::new(Mutex::new(DictationState::Idle));
    let source = Arc::new(Mutex::new(services.source.clone()));
    let controller = Controller::new(
        services,
        tx.clone(),
        Arc::clone(&state),
        Arc::clone(&source),
    );
    tauri::async_runtime::spawn(controller.run(rx));
    ControllerHandle { tx, state, source }
}

struct Recording {
    recorder: Recorder,
    session: u64,
    /// The app the user dictates into and its focused field, read while
    /// recording (CTX-01).
    context: PendingContext,
}

struct Controller {
    hotkeys: Box<dyn HotkeyBackend>,
    bindings: Vec<Binding>,
    push_to_talk_label: String,
    command_mode_label: String,
    inserter: Arc<dyn TextInserter>,
    focus: Arc<dyn FocusInspector>,
    permissions: Arc<dyn Permissions>,
    sounds: Arc<dyn CuePlayer>,
    launcher: Arc<dyn Launcher>,
    settings: Arc<SettingsStore>,
    ui: Arc<dyn DictationUi>,
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
            languages: services.languages,
            clipboard_hold: services.timing.clipboard_hold,
            remote_clipboard_hold: services.timing.remote_clipboard_hold,
        };
        Self {
            hotkeys: services.hotkeys,
            bindings: services.bindings,
            push_to_talk_label: services.push_to_talk_label,
            command_mode_label: services.command_mode_label,
            inserter: services.inserter,
            focus: services.focus,
            permissions: services.permissions,
            sounds: services.sounds,
            launcher: services.launcher,
            settings: services.settings,
            ui: services.ui,
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
                if event.action != Action::PressEnter {
                    self.gesture(Input::Release(event.action));
                }
            }
            ControllerInput::SettingsChanged => self.refresh_bindings(),
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
                self.ui.notice(Notice::warning(format!(
                    "Dictation stops in {}.",
                    minutes(left)
                )));
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
        let spec = lock(&self.source).clone();
        let available = spec.check_available();
        if available.is_ok() && spec.is_microphone() && !self.microphone_allowed() {
            self.gestures.reset();
            return;
        }
        // Capture starts before anything else, so the first word is kept (ENG-04).
        match available.and_then(|()| Recorder::start(spec.open())) {
            Ok(recorder) => {
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
                self.recording = Some(Recording {
                    recorder,
                    session,
                    context,
                });
                self.sounds.play(Cue::Start);
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
            Err(error) => {
                log::warn!("could not start recording from {spec}: {error}");
                self.gestures.reset();
                self.ui.notice(match error {
                    AudioError::NoInputDevice => {
                        Notice::error("No microphone found. Connect one and try again.")
                    }
                    error => Notice::error(format!("Couldn't start recording: {error}")),
                });
            }
        }
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
        let Some(recording) = self.recording.take() else {
            return;
        };
        self.refresh_bindings();
        self.set_state(DictationState::Idle);
        let command = self.gestures.is_command();
        match reason {
            CancelReason::User => {
                self.sounds.play(Cue::Cancel);
                self.ui.notice(Notice::info(if command {
                    "Command cancelled"
                } else {
                    "Dictation cancelled"
                }));
                let tx = self.tx.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    if let Ok(clip) = recording.recorder.finish() {
                        let _ = tx.send(ControllerInput::Cancelled(clip));
                    }
                });
            }
            CancelReason::ShortTap if command => {
                drop(recording);
                self.ui.notice(Notice::info(format!(
                    "Hold {} while you speak a command, or double-press it for hands-free.",
                    self.command_mode_label
                )));
            }
            CancelReason::ShortTap => {
                drop(recording);
                self.ui.notice(Notice::info(format!(
                    "Hold {} to dictate, or double-tap it for hands-free.",
                    self.push_to_talk_label
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
        if let Some(last) = outcomes::report(processed.outcome, self.ui.as_ref()) {
            self.last = Some(last);
        }
    }

    /// The Press Enter shortcut (CORE-15); extra modifiers pass through.
    fn press_enter(&self, extra: Modifiers) {
        let inserter = Arc::clone(&self.inserter);
        let ui = Arc::clone(&self.ui);
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(error) = inserter.press(SyntheticKey::Enter, extra) {
                ui.notice(Notice::error(format!("Couldn't press Enter: {error}")));
            }
        });
    }

    /// The Command Mode shortcut while it is off (CMD-06): a notice with a
    /// way to the setting, at most once a day and three times in all.
    fn command_mode_off(&self) {
        if self.recording.is_some() || self.gestures.is_busy() {
            return;
        }
        let settings = self.settings.get();
        let today = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs())
            / SECONDS_PER_DAY)
            .to_string();
        let notices = &settings.notices;
        if notices.command_mode_off_count >= COMMAND_OFF_NOTICES
            || notices.command_mode_off_day == today
        {
            return;
        }
        let patch = serde_json::json!({"notices": {
            "commandModeOffCount": notices.command_mode_off_count + 1,
            "commandModeOffDay": today,
        }});
        if let Err(error) = self.settings.update(&patch) {
            log::warn!("could not count the notice: {error}");
        }
        self.ui.notice(
            Notice::info("Command mode is toggled off")
                .with_action(NoticeAction::OpenExperimentalSettings),
        );
    }

    fn poll_audio(&mut self) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        match recording.recorder.poll() {
            Ok(samples) => self.ui.level(level::display_level(level::rms(samples))),
            Err(error) => {
                log::warn!("recording failed: {error}");
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
            let _ = tx.send(ControllerInput::Shortcut(event));
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

    /// Esc (and Backspace in Command Mode) is only swallowed while a session
    /// records (CORE-06, CMD-04), and until its key-up, so no half of the
    /// press reaches the app. The user's own shortcuts come from the settings.
    fn refresh_bindings(&mut self) {
        let moment = Moment {
            cancelable: self.recording.is_some() || self.cancel_held,
            command: self.gestures.is_command(),
        };
        let bindings = bindings::active(&self.bindings, &self.settings.get(), moment);
        self.hotkeys.set_bindings(&bindings);
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
