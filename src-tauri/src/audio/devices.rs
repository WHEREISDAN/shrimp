//! Input devices (MIC-02..08): what is connected, of what kind, and how to
//! record from one. The OS backend implements [`DeviceProvider`]; tests and
//! debug builds can use [`FakeDevices`] instead, with simulated plugging,
//! silence, slow opening and disconnects.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::{AudioError, AudioFormat, AudioSink, AudioSource, MicrophoneSource, read_wav};

/// How a device is connected, as far as choosing one goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceKind {
    BuiltIn,
    /// USB, headphone jack, Thunderbolt and similar.
    Wired,
    Bluetooth,
    /// Software devices (BlackHole, Krisp, Loopback…), never chosen automatically (MIC-06).
    Virtual,
}

/// Mirrors `InputDevice` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDevice {
    /// Stable across reconnects (Core Audio's device UID).
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
}

impl InputDevice {
    pub fn is_airpods(&self) -> bool {
        self.kind == DeviceKind::Bluetooth && self.name.to_lowercase().contains("airpods")
    }
}

/// Software inputs that often report a physical transport.
const VIRTUAL_NAMES: [&str; 14] = [
    "blackhole",
    "krisp",
    "loopback",
    "soundflower",
    "zoomaudiodevice",
    "zoom audio",
    "teams audio",
    "background music",
    "vb-cable",
    "voicemeeter",
    "ndi audio",
    "obs ",
    "aggregate device",
    "multi-output",
];

/// The kind of a device from its transport and name.
pub fn classify(name: &str, transport: DeviceKind) -> DeviceKind {
    let lower = name.to_lowercase();
    if VIRTUAL_NAMES.iter().any(|known| lower.contains(known)) {
        DeviceKind::Virtual
    } else {
        transport
    }
}

pub trait DeviceProvider: Send + Sync {
    /// Input devices connected now.
    fn input_devices(&self) -> Vec<InputDevice>;
    /// The system's default input, by id.
    fn default_input(&self) -> Option<String>;
    /// A MacBook's lid is closed, so its built-in microphone hears nothing (MIC-05).
    fn lid_closed(&self) -> bool {
        false
    }
    /// A source recording from `device`.
    fn open(&self, device: &InputDevice) -> Box<dyn AudioSource>;
}

/// The OS's devices through cpal, for platforms without a richer backend.
pub struct CpalDevices;

impl DeviceProvider for CpalDevices {
    fn input_devices(&self) -> Vec<InputDevice> {
        super::mic::input_names()
            .into_iter()
            .map(|name| InputDevice {
                id: name.clone(),
                kind: classify(&name, DeviceKind::Wired),
                name,
            })
            .collect()
    }

    fn default_input(&self) -> Option<String> {
        super::mic::default_input_name()
    }

    fn open(&self, device: &InputDevice) -> Box<dyn AudioSource> {
        Box::new(MicrophoneSource::new(Some(device.name.clone())))
    }
}

/// What a fake device does once opened.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum FakeBehaviour {
    /// A steady tone at this amplitude.
    Tone { amplitude: f32 },
    /// A WAV file, looped.
    Wav { path: PathBuf },
    /// Digital silence, like a muted microphone.
    Silent,
    /// Opens after this long.
    SlowOpen { millis: u64 },
    /// Opens but never delivers audio.
    Dead,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FakeDevice {
    #[serde(flatten)]
    pub device: InputDevice,
    pub behaviour: FakeBehaviour,
}

/// Devices for tests and debug builds. Removing a device disconnects any
/// source recording from it.
#[derive(Default)]
pub struct FakeDevices {
    devices: Mutex<Vec<FakeDevice>>,
    default: Mutex<Option<String>>,
    lid_closed: AtomicBool,
}

impl FakeDevices {
    pub fn set(&self, devices: Vec<FakeDevice>) {
        *lock(&self.devices) = devices;
    }

    pub fn add(&self, device: FakeDevice) {
        lock(&self.devices).push(device);
    }

    pub fn remove(&self, id: &str) {
        lock(&self.devices).retain(|fake| fake.device.id != id);
    }

    pub fn set_default(&self, id: Option<&str>) {
        *lock(&self.default) = id.map(str::to_owned);
    }

    pub fn set_lid_closed(&self, closed: bool) {
        self.lid_closed.store(closed, Ordering::SeqCst);
    }

    fn connected(&self, id: &str) -> bool {
        lock(&self.devices).iter().any(|fake| fake.device.id == id)
    }
}

impl DeviceProvider for Arc<FakeDevices> {
    fn input_devices(&self) -> Vec<InputDevice> {
        lock(&self.devices)
            .iter()
            .map(|fake| fake.device.clone())
            .collect()
    }

    fn default_input(&self) -> Option<String> {
        lock(&self.default).clone()
    }

    fn lid_closed(&self) -> bool {
        self.lid_closed.load(Ordering::SeqCst)
    }

    fn open(&self, device: &InputDevice) -> Box<dyn AudioSource> {
        let behaviour = lock(&self.devices)
            .iter()
            .find(|fake| fake.device.id == device.id)
            .map(|fake| fake.behaviour.clone());
        Box::new(FakeSource {
            devices: Arc::clone(self),
            id: device.id.clone(),
            behaviour,
            worker: None,
        })
    }
}

/// Debug builds: the OS's devices, or fake ones an agent sets through
/// `dev_set_microphones` to simulate plugging, silence and failures.
pub struct SwitchableDevices {
    native: Arc<dyn DeviceProvider>,
    fake: Arc<FakeDevices>,
    use_fake: AtomicBool,
}

impl SwitchableDevices {
    pub fn new(native: Arc<dyn DeviceProvider>) -> Self {
        Self {
            native,
            fake: Arc::new(FakeDevices::default()),
            use_fake: AtomicBool::new(false),
        }
    }

    /// Uses `devices` from now on, or the OS's devices again with `None`.
    pub fn use_fake(
        &self,
        devices: Option<Vec<FakeDevice>>,
        default: Option<&str>,
        lid_closed: bool,
    ) {
        self.use_fake.store(devices.is_some(), Ordering::SeqCst);
        self.fake.set(devices.unwrap_or_default());
        self.fake.set_default(default);
        self.fake.set_lid_closed(lid_closed);
    }

    fn active(&self) -> Arc<dyn DeviceProvider> {
        if self.use_fake.load(Ordering::SeqCst) {
            Arc::new(Arc::clone(&self.fake))
        } else {
            Arc::clone(&self.native)
        }
    }
}

impl DeviceProvider for SwitchableDevices {
    fn input_devices(&self) -> Vec<InputDevice> {
        self.active().input_devices()
    }

    fn default_input(&self) -> Option<String> {
        self.active().default_input()
    }

    fn lid_closed(&self) -> bool {
        self.active().lid_closed()
    }

    fn open(&self, device: &InputDevice) -> Box<dyn AudioSource> {
        self.active().open(device)
    }
}

const FAKE_RATE: u32 = 16_000;
const FAKE_CHUNK: Duration = Duration::from_millis(20);

struct FakeSource {
    devices: Arc<FakeDevices>,
    id: String,
    behaviour: Option<FakeBehaviour>,
    worker: Option<(Arc<AtomicBool>, JoinHandle<()>)>,
}

impl AudioSource for FakeSource {
    fn describe(&self) -> String {
        format!("fake microphone {}", self.id)
    }

    fn start(&mut self, sink: AudioSink) -> Result<Option<AudioFormat>, AudioError> {
        let behaviour = self.behaviour.clone().ok_or(AudioError::NoInputDevice)?;
        let samples = match &behaviour {
            FakeBehaviour::Wav { path } => {
                let (format, samples) = read_wav(path)?;
                super::AudioClip::from_interleaved(format, &samples)?.into_samples()
            }
            _ => Vec::new(),
        };
        let stop = Arc::new(AtomicBool::new(false));
        let devices = Arc::clone(&self.devices);
        let id = self.id.clone();
        let worker_stop = Arc::clone(&stop);
        let handle = std::thread::Builder::new()
            .name("shrimp-fake-mic".into())
            .spawn(move || {
                let fake = FakeRun {
                    devices,
                    id,
                    stop: worker_stop,
                    sink,
                };
                fake.run(&behaviour, &samples);
            })
            .map_err(|err| AudioError::Device(format!("could not start a fake device: {err}")))?;
        self.worker = Some((stop, handle));
        Ok(None)
    }

    fn stop(&mut self) {
        if let Some((stop, handle)) = self.worker.take() {
            stop.store(true, Ordering::SeqCst);
            let _ = handle.join();
        }
    }
}

impl Drop for FakeSource {
    fn drop(&mut self) {
        self.stop();
    }
}

struct FakeRun {
    devices: Arc<FakeDevices>,
    id: String,
    stop: Arc<AtomicBool>,
    sink: AudioSink,
}

impl FakeRun {
    fn run(&self, behaviour: &FakeBehaviour, wav: &[f32]) {
        if let FakeBehaviour::SlowOpen { millis } = behaviour
            && !self.wait(Duration::from_millis(*millis))
        {
            return;
        }
        self.sink.format(AudioFormat {
            sample_rate: FAKE_RATE,
            channels: 1,
        });
        let chunk = (FAKE_RATE as usize * FAKE_CHUNK.as_millis() as usize) / 1000;
        let started = Instant::now();
        for index in 0usize.. {
            if !self.wait_until(started + FAKE_CHUNK * (index as u32 + 1)) {
                return;
            }
            let samples: Vec<f32> = match behaviour {
                FakeBehaviour::Tone { amplitude } => (0..chunk)
                    .map(|i| amplitude * ((index * chunk + i) as f32 * 0.07).sin())
                    .collect(),
                FakeBehaviour::Wav { .. } if !wav.is_empty() => (0..chunk)
                    .map(|i| wav[(index * chunk + i) % wav.len()])
                    .collect(),
                FakeBehaviour::Dead => continue,
                _ => vec![0.0; chunk],
            };
            if !self.sink.push(samples) {
                return;
            }
        }
    }

    /// Sleeps until `deadline`; false when stopped or unplugged meanwhile.
    fn wait_until(&self, deadline: Instant) -> bool {
        loop {
            if self.stop.load(Ordering::SeqCst) {
                return false;
            }
            if !self.devices.connected(&self.id) {
                self.sink.fail(AudioError::Disconnected);
                return false;
            }
            let now = Instant::now();
            if now >= deadline {
                return true;
            }
            std::thread::sleep((deadline - now).min(Duration::from_millis(10)));
        }
    }

    fn wait(&self, duration: Duration) -> bool {
        self.wait_until(Instant::now() + duration)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::Recorder;

    fn device(id: &str, kind: DeviceKind, behaviour: FakeBehaviour) -> FakeDevice {
        FakeDevice {
            device: InputDevice {
                id: id.into(),
                name: id.into(),
                kind,
            },
            behaviour,
        }
    }

    #[test]
    fn virtual_devices_are_recognised_by_name() {
        assert_eq!(
            classify("BlackHole 2ch", DeviceKind::Wired),
            DeviceKind::Virtual
        );
        assert_eq!(
            classify("krisp microphone", DeviceKind::Wired),
            DeviceKind::Virtual
        );
        assert_eq!(
            classify("MacBook Pro Microphone", DeviceKind::BuiltIn),
            DeviceKind::BuiltIn
        );
        let pods = InputDevice {
            id: "x".into(),
            name: "Dan's AirPods Pro".into(),
            kind: DeviceKind::Bluetooth,
        };
        assert!(pods.is_airpods());
    }

    #[test]
    fn a_fake_tone_records_and_unplugging_disconnects() {
        let fakes = Arc::new(FakeDevices::default());
        fakes.add(device(
            "usb",
            DeviceKind::Wired,
            FakeBehaviour::Tone { amplitude: 0.3 },
        ));
        let source = fakes.open(&fakes.input_devices()[0]);
        let mut recorder = Recorder::start(source).unwrap();
        std::thread::sleep(Duration::from_millis(150));
        assert!(!recorder.poll().unwrap().is_empty());
        assert!(recorder.is_open());
        fakes.remove("usb");
        std::thread::sleep(Duration::from_millis(80));
        assert!(matches!(recorder.poll(), Err(AudioError::Disconnected)));
    }

    #[test]
    fn a_slow_device_opens_late() {
        let fakes = Arc::new(FakeDevices::default());
        fakes.add(device(
            "slow",
            DeviceKind::Wired,
            FakeBehaviour::SlowOpen { millis: 200 },
        ));
        let mut recorder = Recorder::start(fakes.open(&fakes.input_devices()[0])).unwrap();
        recorder.poll().unwrap();
        assert!(!recorder.is_open());
        std::thread::sleep(Duration::from_millis(300));
        recorder.poll().unwrap();
        assert!(recorder.is_open());
    }
}
