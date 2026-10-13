use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig};

use super::{AudioError, AudioFormat, AudioSink, AudioSource};

/// Records from a microphone in its native format (MIC-01). The cpal stream
/// lives on its own thread because CoreAudio streams are not `Send`; the
/// callback only converts samples to f32 and hands them to the sink.
pub struct MicrophoneSource {
    device: Selector,
    worker: Option<Worker>,
}

/// Which input device to open.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Selector {
    Default,
    Name(String),
    /// The backend's device id (Core Audio's UID).
    Id(String),
}

struct Worker {
    stop: Sender<()>,
    handle: JoinHandle<()>,
}

impl MicrophoneSource {
    /// `None` records from the system default input.
    pub fn new(device: Option<String>) -> Self {
        Self {
            device: device.map_or(Selector::Default, Selector::Name),
            worker: None,
        }
    }

    /// Records from the device with this backend id (Core Audio's UID).
    pub fn with_id(id: String) -> Self {
        Self {
            device: Selector::Id(id),
            worker: None,
        }
    }
}

/// Whether the microphone exists, without opening it (and without any
/// permission prompt).
pub fn microphone_available(name: Option<&str>) -> bool {
    let selector = name.map_or(Selector::Default, |name| Selector::Name(name.to_owned()));
    find_device(&selector).is_ok()
}

/// Names of the input devices cpal lists.
pub fn input_names() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|devices| {
            devices
                .filter_map(|device| device.description().ok())
                .map(|description| description.name().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

pub fn default_input_name() -> Option<String> {
    let device = cpal::default_host().default_input_device()?;
    Some(device.description().ok()?.name().to_owned())
}

impl AudioSource for MicrophoneSource {
    fn describe(&self) -> String {
        match &self.device {
            Selector::Name(name) => format!("microphone {name}"),
            Selector::Id(id) => format!("microphone {id}"),
            Selector::Default => "default microphone".into(),
        }
    }

    fn start(&mut self, sink: AudioSink) -> Result<Option<AudioFormat>, AudioError> {
        if self.worker.is_some() {
            return Err(AudioError::AlreadyStarted);
        }
        let (ready_tx, ready_rx) = mpsc::channel();
        let (stop, stop_rx) = mpsc::channel::<()>();
        let device = self.device.clone();
        let handle = std::thread::Builder::new()
            .name("shrimp-microphone".into())
            .spawn(move || match open_stream(&device, sink) {
                Ok((stream, format)) => {
                    let _ = ready_tx.send(Ok(format));
                    // Keep the stream alive until stopped.
                    let _ = stop_rx.recv();
                    drop(stream);
                }
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                }
            })
            .map_err(|err| {
                AudioError::Device(format!("could not start microphone thread: {err}"))
            })?;
        let format = ready_rx
            .recv()
            .map_err(|_| AudioError::Disconnected)
            .and_then(|result| result);
        match format {
            Ok(format) => {
                self.worker = Some(Worker { stop, handle });
                Ok(Some(format))
            }
            Err(error) => {
                let _ = handle.join();
                Err(error)
            }
        }
    }

    fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.stop.send(());
            let _ = worker.handle.join();
        }
    }
}

impl Drop for MicrophoneSource {
    fn drop(&mut self) {
        self.stop();
    }
}

fn find_device(selector: &Selector) -> Result<Device, AudioError> {
    let host = cpal::default_host();
    let devices = || {
        host.input_devices()
            .map_err(|err| AudioError::Device(format!("could not list microphones: {err}")))
    };
    let device = match selector {
        Selector::Default => host.default_input_device(),
        Selector::Name(name) => devices()?.find(|device| {
            device
                .description()
                .is_ok_and(|description| description.name() == name)
        }),
        Selector::Id(id) => {
            devices()?.find(|device| device.id().is_ok_and(|found| found.id() == id))
        }
    };
    // A Mac without any input hardware can still report a default input
    // device that has no usable configuration.
    device
        .filter(|device| device.default_input_config().is_ok())
        .ok_or(AudioError::NoInputDevice)
}

fn open_stream(selector: &Selector, sink: AudioSink) -> Result<(Stream, AudioFormat), AudioError> {
    let device = find_device(selector)?;
    let supported = device
        .default_input_config()
        .map_err(|err| AudioError::Device(format!("microphone unavailable: {err}")))?;
    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let format = AudioFormat {
        sample_rate: config.sample_rate,
        channels: config.channels,
    };
    let stream = match sample_format {
        SampleFormat::F32 => build::<f32>(&device, &config, sink),
        SampleFormat::I16 => build::<i16>(&device, &config, sink),
        SampleFormat::I32 => build::<i32>(&device, &config, sink),
        SampleFormat::U16 => build::<u16>(&device, &config, sink),
        SampleFormat::I8 => build::<i8>(&device, &config, sink),
        SampleFormat::U8 => build::<u8>(&device, &config, sink),
        SampleFormat::F64 => build::<f64>(&device, &config, sink),
        other => {
            return Err(AudioError::UnsupportedFormat(format!(
                "microphone sample format {other}"
            )));
        }
    }?;
    stream
        .play()
        .map_err(|err| AudioError::Device(format!("could not start the microphone: {err}")))?;
    Ok((stream, format))
}

fn build<T>(device: &Device, config: &StreamConfig, sink: AudioSink) -> Result<Stream, AudioError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let error_sink = sink.clone();
    device
        .build_input_stream(
            *config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                sink.push(
                    data.iter()
                        .map(|&sample| f32::from_sample(sample))
                        .collect(),
                );
            },
            move |err: cpal::Error| match err.kind() {
                cpal::ErrorKind::DeviceNotAvailable => error_sink.fail(AudioError::Disconnected),
                _ => log::warn!("microphone stream: {err}"),
            },
            None,
        )
        .map_err(|err| AudioError::Device(format!("could not open the microphone: {err}")))
}
