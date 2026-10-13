//! Records from the best microphone and, when it fails or is unplugged,
//! carries on with the next one without a break (MIC-03). Devices open on a
//! background thread, so a slow microphone never blocks the dictation.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::audio::{
    AudioError, AudioEvent, AudioFormat, AudioSink, AudioSource, DeviceProvider, InputDevice,
};
use crate::t;

/// Reported as the source moves between devices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Switch {
    /// `from` failed or was unplugged; recording goes on from `to`.
    Fallback { from: InputDevice, to: InputDevice },
}

type OnSwitch = Arc<dyn Fn(Switch) + Send + Sync>;

pub struct FallbackSource {
    provider: Arc<dyn DeviceProvider>,
    candidates: Vec<InputDevice>,
    on_switch: OnSwitch,
    worker: Option<(Arc<AtomicBool>, JoinHandle<()>)>,
}

impl FallbackSource {
    pub fn new(
        provider: Arc<dyn DeviceProvider>,
        candidates: Vec<InputDevice>,
        on_switch: OnSwitch,
    ) -> Self {
        Self {
            provider,
            candidates,
            on_switch,
            worker: None,
        }
    }
}

impl AudioSource for FallbackSource {
    fn describe(&self) -> String {
        let names: Vec<&str> = self
            .candidates
            .iter()
            .map(|device| device.name.as_str())
            .collect();
        format!("microphones {}", names.join(", then "))
    }

    fn start(&mut self, sink: AudioSink) -> Result<Option<AudioFormat>, AudioError> {
        if self.candidates.is_empty() {
            return Err(AudioError::NoInputDevice);
        }
        let stop = Arc::new(AtomicBool::new(false));
        let run = Run {
            provider: Arc::clone(&self.provider),
            candidates: self.candidates.clone(),
            on_switch: Arc::clone(&self.on_switch),
            stop: Arc::clone(&stop),
            sink,
        };
        let handle = std::thread::Builder::new()
            .name("shrimp-microphones".into())
            .spawn(move || run.run())
            .map_err(|err| AudioError::Device(t!("notice.audio.startFailed", error = err)))?;
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

impl Drop for FallbackSource {
    fn drop(&mut self) {
        self.stop();
    }
}

struct Run {
    provider: Arc<dyn DeviceProvider>,
    candidates: Vec<InputDevice>,
    on_switch: OnSwitch,
    stop: Arc<AtomicBool>,
    sink: AudioSink,
}

enum Outcome {
    /// Stopped, ended or nobody listens any more.
    Done,
    Failed(AudioError),
}

impl Run {
    fn run(&self) {
        let mut last_error = AudioError::NoInputDevice;
        for (index, device) in self.candidates.iter().enumerate() {
            match self.record(device) {
                Outcome::Done => return,
                Outcome::Failed(error) => {
                    log::warn!("{} failed: {error}", device.name);
                    last_error = error;
                }
            }
            if let Some(next) = self.candidates.get(index + 1) {
                (self.on_switch)(Switch::Fallback {
                    from: device.clone(),
                    to: next.clone(),
                });
            }
        }
        self.sink.fail(last_error);
    }

    /// Forwards one device's audio until it stops, ends or fails.
    fn record(&self, device: &InputDevice) -> Outcome {
        let mut source = self.provider.open(device);
        let (inner, events) = AudioSink::channel();
        match source.start(inner) {
            Ok(Some(format)) => {
                self.sink.format(format);
            }
            Ok(None) => {}
            Err(error) => return Outcome::Failed(error),
        }
        let outcome = loop {
            if self.stop.load(Ordering::SeqCst) {
                break Outcome::Done;
            }
            match events.recv_timeout(Duration::from_millis(20)) {
                Ok(AudioEvent::Format(format)) => {
                    self.sink.format(format);
                }
                Ok(AudioEvent::Samples(samples)) => {
                    if !self.sink.push(samples) {
                        break Outcome::Done;
                    }
                }
                Ok(AudioEvent::Ended) => {
                    self.sink.end();
                    break Outcome::Done;
                }
                Ok(AudioEvent::Failed(error)) => break Outcome::Failed(error),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    break Outcome::Failed(AudioError::Disconnected);
                }
            }
        };
        source.stop();
        outcome
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::audio::{DeviceKind, FakeBehaviour, FakeDevice, FakeDevices, Recorder};

    fn fake(id: &str, behaviour: FakeBehaviour) -> FakeDevice {
        FakeDevice {
            device: InputDevice {
                id: id.into(),
                name: id.into(),
                kind: DeviceKind::Wired,
            },
            behaviour,
        }
    }

    #[test]
    fn unplugging_the_microphone_carries_on_with_the_next_without_a_break() {
        let fakes = Arc::new(FakeDevices::default());
        fakes.add(fake("usb", FakeBehaviour::Tone { amplitude: 0.3 }));
        fakes.add(fake("builtin", FakeBehaviour::Tone { amplitude: 0.2 }));
        let switches = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&switches);
        let provider: Arc<dyn DeviceProvider> = Arc::new(Arc::clone(&fakes));
        let source = FallbackSource::new(
            provider,
            fakes.input_devices(),
            Arc::new(move |switch| seen.lock().unwrap().push(switch)),
        );
        let mut recorder = Recorder::start(Box::new(source)).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        recorder.poll().unwrap();
        fakes.remove("usb");
        std::thread::sleep(Duration::from_millis(400));
        recorder.poll().unwrap();
        let clip = recorder.finish().unwrap();
        let seconds = clip.duration().as_secs_f32();
        assert!(seconds > 0.55, "audio from both microphones: {seconds} s");
        let switches = switches.lock().unwrap();
        assert_eq!(switches.len(), 1);
        let Switch::Fallback { from, to } = &switches[0];
        assert_eq!((from.id.as_str(), to.id.as_str()), ("usb", "builtin"));
    }

    #[test]
    fn when_every_microphone_fails_the_recording_fails() {
        let fakes = Arc::new(FakeDevices::default());
        fakes.add(fake("usb", FakeBehaviour::Tone { amplitude: 0.3 }));
        let provider: Arc<dyn DeviceProvider> = Arc::new(Arc::clone(&fakes));
        let source = FallbackSource::new(provider, fakes.input_devices(), Arc::new(|_| {}));
        let mut recorder = Recorder::start(Box::new(source)).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        fakes.remove("usb");
        std::thread::sleep(Duration::from_millis(200));
        assert!(matches!(recorder.poll(), Err(AudioError::Disconnected)));
    }
}
