//! Microphones (MIC-02..08): which one a dictation records from, falling
//! back mid-dictation, the ranking, hot-plug prompts, Bluetooth advice and
//! the picker's live level meter. Devices come from a [`DeviceProvider`],
//! so tests and debug builds can use fake ones.

pub mod choice;
mod fallback;
pub mod monitor;
mod preview;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::json;

pub use choice::{MicrophoneEntry, MicrophoneList};
use fallback::{FallbackSource, Switch};
pub use monitor::{MicChange, MicMonitor, MicWarning};
use preview::Preview;

use crate::audio::{AudioError, AudioSource, DeviceKind, DeviceProvider, InputDevice};
use crate::dictation::DictationUi;
use crate::notify::{Notice, NoticeAction, NoticeCategory};
use crate::settings::{MicrophoneSettings, SettingsStore};

/// How often connected devices are checked (hot-plug).
pub const POLL_INTERVAL: Duration = Duration::from_secs(2);
const BLUETOOTH_ADVICE_LIMIT: u32 = 3;
const WEEK: u64 = 7 * 24 * 60 * 60;

type OnList = Box<dyn Fn(&MicrophoneList) + Send + Sync>;

/// A microphone source for a dictation, and the device it starts with.
pub struct Opened {
    pub source: Box<dyn AudioSource>,
    pub device: InputDevice,
}

#[derive(Default)]
struct State {
    connected: Vec<InputDevice>,
    lid_closed: bool,
    recording: bool,
    /// A new microphone waiting for its Switch / Don't switch prompt.
    pending: Option<InputDevice>,
    /// Asked about this session already.
    prompted: HashSet<String>,
}

pub struct Microphones {
    provider: Arc<dyn DeviceProvider>,
    settings: Arc<SettingsStore>,
    ui: Arc<dyn DictationUi>,
    state: Mutex<State>,
    on_list: OnList,
    preview: Mutex<Option<Preview>>,
}

impl Microphones {
    pub fn new(
        provider: Arc<dyn DeviceProvider>,
        settings: Arc<SettingsStore>,
        ui: Arc<dyn DictationUi>,
        on_list: OnList,
    ) -> Arc<Self> {
        let microphones = Arc::new(Self {
            provider,
            settings,
            ui,
            state: Mutex::new(State::default()),
            on_list,
            preview: Mutex::new(None),
        });
        microphones.poll();
        microphones
    }

    fn settings(&self) -> MicrophoneSettings {
        self.settings.get().microphone
    }

    pub fn list(&self) -> MicrophoneList {
        let connected = lock(&self.state).connected.clone();
        choice::list(
            &connected,
            &self.settings(),
            self.provider.lid_closed(),
            self.provider.default_input().as_deref(),
        )
    }

    fn order(&self) -> Vec<InputDevice> {
        let connected = self.provider.input_devices();
        choice::order(
            &connected,
            &self.settings(),
            self.provider.lid_closed(),
            self.provider.default_input().as_deref(),
        )
    }

    /// The source for a dictation: the best microphone, then the others as
    /// fallbacks. A selected microphone that can't be used is reported.
    pub fn open(self: &Arc<Self>) -> Result<Opened, AudioError> {
        let order = self.order();
        let settings = self.settings();
        let unavailable = settings
            .selected
            .as_deref()
            .filter(|id| order.first().is_none_or(|first| first.id != *id));
        let Some(first) = order.first().cloned() else {
            self.ui.notice(match unavailable {
                Some(_) => Notice::error("Selected microphone is unavailable")
                    .with_action(NoticeAction::ChooseMicrophone),
                None => Notice::error("No microphone detected")
                    .with_action(NoticeAction::OpenSoundSettings),
            });
            return Err(AudioError::NoInputDevice);
        };
        if let Some(id) = unavailable {
            let reason = self
                .provider
                .input_devices()
                .iter()
                .find(|device| device.id == id)
                .and_then(|device| choice::unavailable(device, self.provider.lid_closed()))
                .map(|reason| format!(" ({reason})"))
                .unwrap_or_default();
            self.ui.notice(
                Notice::warning(format!(
                    "Selected microphone is unavailable{reason}. Using {}.",
                    first.name
                ))
                .with_action(NoticeAction::ChooseMicrophone),
            );
        }
        let ui = Arc::clone(&self.ui);
        let source = FallbackSource::new(
            Arc::clone(&self.provider),
            order,
            Arc::new(move |switch| match switch {
                Switch::Fallback { from, to } => ui.notice(Notice::warning(format!(
                    "{} disconnected. Switched to {}.",
                    from.name, to.name
                ))),
            }),
        );
        log::info!("recording from {} ({:?})", first.name, first.kind);
        Ok(Opened {
            source: Box::new(source),
            device: first,
        })
    }

    /// A dictation started recording from `device`.
    pub fn recording_started(&self, device: &InputDevice) {
        lock(&self.state).recording = true;
        self.stop_preview();
        if device.kind == DeviceKind::Bluetooth {
            self.bluetooth_advice(device);
        }
    }

    /// The recording ended; a waiting new-microphone prompt can show now.
    pub fn recording_ended(&self) {
        let pending = {
            let mut state = lock(&self.state);
            state.recording = false;
            state.pending.clone()
        };
        if let Some(device) = pending {
            self.prompt(&device);
        }
    }

    /// Advice to use another microphone than a Bluetooth one, at most 3
    /// times and once a week, until "Don't show again" (MIC-08).
    fn bluetooth_advice(&self, device: &InputDevice) {
        let notices = self.settings.get().notices;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        if notices.bluetooth_advice_off
            || notices.bluetooth_advice_count >= BLUETOOTH_ADVICE_LIMIT
            || now.saturating_sub(notices.bluetooth_advice_at) < WEEK
        {
            return;
        }
        let patch = json!({"notices": {
            "bluetoothAdviceCount": notices.bluetooth_advice_count + 1,
            "bluetoothAdviceAt": now,
        }});
        if let Err(error) = self.settings.update(&patch) {
            log::warn!("could not count the advice: {error}");
        }
        self.ui.notice(
            Notice::warning(format!(
                "Bluetooth microphones like {} sound muffled and can be slow to start. A \
                 built-in or wired microphone works better.",
                device.name
            ))
            .with_action(NoticeAction::SwitchMicrophone)
            .with_action(NoticeAction::DismissBluetoothAdvice)
            .in_category(NoticeCategory::Suggestion),
        );
    }

    pub fn dismiss_bluetooth_advice(&self) -> Result<(), String> {
        self.update(json!({"notices": {"bluetoothAdviceOff": true}}))
    }

    /// Checks what is connected (hot-plug): new physical microphones join
    /// the ranking, after a Switch / Don't switch prompt once a ranking
    /// exists (MIC-04; not for AirPods).
    pub fn poll(&self) {
        let connected = self.provider.input_devices();
        let lid_closed = self.provider.lid_closed();
        let added: Vec<InputDevice> = {
            let mut state = lock(&self.state);
            if state.connected == connected && state.lid_closed == lid_closed {
                return;
            }
            state.lid_closed = lid_closed;
            let added = connected
                .iter()
                .filter(|device| !state.connected.iter().any(|known| known.id == device.id))
                .cloned()
                .collect();
            state.connected = connected.clone();
            added
        };
        let settings = self.settings();
        let new: Vec<InputDevice> = added
            .into_iter()
            .filter(|device| device.kind != DeviceKind::Virtual)
            .filter(|device| !settings.ranking.iter().any(|known| known.id == device.id))
            .collect();
        if settings.ranking.is_empty() {
            let order = choice::order(
                &connected,
                &settings,
                self.provider.lid_closed(),
                self.provider.default_input().as_deref(),
            );
            let ranking: Vec<_> = order
                .iter()
                .filter(|device| device.kind != DeviceKind::Virtual)
                .fold(Vec::new(), |ranking, device| {
                    choice::ranked_with(&ranking, device, false)
                });
            if !ranking.is_empty() {
                self.save_ranking(&ranking);
            }
        } else {
            for device in new {
                if device.is_airpods() {
                    let ranking = choice::ranked_with(&self.settings().ranking, &device, false);
                    self.save_ranking(&ranking);
                } else {
                    self.new_microphone(device);
                }
            }
        }
        (self.on_list)(&self.list());
    }

    fn new_microphone(&self, device: InputDevice) {
        let recording = {
            let mut state = lock(&self.state);
            if !state.prompted.insert(device.id.clone()) {
                return;
            }
            state.pending = Some(device.clone());
            state.recording
        };
        if !recording {
            self.prompt(&device);
        }
    }

    fn prompt(&self, device: &InputDevice) {
        self.ui.notice(
            Notice::info(format!("{} is connected. Switch to it?", device.name))
                .with_action(NoticeAction::SwitchToNewMicrophone)
                .with_action(NoticeAction::KeepMicrophone),
        );
    }

    /// The new-microphone prompt's answer: Switch puts it first (or selects
    /// it when a microphone is chosen), Don't switch ranks it last.
    pub fn answer_new_microphone(&self, switch: bool) -> Result<(), String> {
        let Some(device) = lock(&self.state).pending.take() else {
            return Ok(());
        };
        let settings = self.settings();
        let ranking = choice::ranked_with(&settings.ranking, &device, switch);
        let selected = match (&settings.selected, switch) {
            (Some(_), true) => json!(device.id),
            (selected, _) => json!(selected),
        };
        self.update(json!({"microphone": {"ranking": ranking, "selected": selected}}))
    }

    /// Chooses a microphone, or Auto-detect with `None`.
    pub fn select(&self, id: Option<String>) -> Result<(), String> {
        self.update(json!({"microphone": {"selected": id}}))
    }

    pub fn rank(&self, id: &str, up: bool) -> Result<(), String> {
        let ranking = choice::moved(&self.settings().ranking, id, up);
        self.update(json!({"microphone": {"ranking": ranking}}))
    }

    /// Forgets a ranked microphone; a selected one goes back to Auto-detect.
    pub fn forget(&self, id: &str) -> Result<(), String> {
        let settings = self.settings();
        let ranking: Vec<_> = settings
            .ranking
            .into_iter()
            .filter(|known| known.id != id)
            .collect();
        let selected = settings.selected.filter(|selected| selected != id);
        self.update(json!({"microphone": {"ranking": ranking, "selected": selected}}))
    }

    fn save_ranking(&self, ranking: &[crate::settings::KnownMicrophone]) {
        if let Err(error) = self.update(json!({"microphone": {"ranking": ranking}})) {
            log::warn!("could not save the microphone ranking: {error}");
        }
    }

    fn update(&self, patch: serde_json::Value) -> Result<(), String> {
        self.settings
            .update(&patch)
            .map_err(|error| error.to_string())?;
        (self.on_list)(&self.list());
        Ok(())
    }

    /// Shows the live level of the microphone a dictation would use, until
    /// stopped or a dictation starts (MIC-02).
    pub fn start_preview(&self, on_level: Box<dyn Fn(f32) + Send + Sync>) {
        if lock(&self.state).recording {
            return;
        }
        let device = self.order().into_iter().next();
        *lock(&self.preview) =
            device.map(|device| Preview::start(self.provider.open(&device), on_level));
    }

    pub fn stop_preview(&self) {
        lock(&self.preview).take();
    }

    /// Checks for plugged and unplugged microphones until the app quits.
    pub fn watch(self: Arc<Self>) {
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(POLL_INTERVAL);
            loop {
                interval.tick().await;
                let microphones = Arc::clone(&self);
                let _ = tokio::task::spawn_blocking(move || microphones.poll()).await;
            }
        });
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}
