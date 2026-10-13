use std::sync::mpsc::{self, Receiver, Sender};

use super::AudioError;

/// Format of the samples a source pushes. Samples are always interleaved f32
/// in `[-1.0, 1.0]`; sources convert from their native sample type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug)]
pub enum AudioEvent {
    /// Interleaved samples in the format returned by [`AudioSource::start`].
    Samples(Vec<f32>),
    /// The source ran out of audio on its own (end of a file).
    Ended,
    /// The source failed and will push nothing more.
    Failed(AudioError),
}

/// Where a running source delivers audio. Cheap to clone and `Send`, so a
/// source can move it into its capture thread or device callback.
#[derive(Debug, Clone)]
pub struct AudioSink {
    tx: Sender<AudioEvent>,
}

impl AudioSink {
    pub fn channel() -> (Self, Receiver<AudioEvent>) {
        let (tx, rx) = mpsc::channel();
        (Self { tx }, rx)
    }

    /// Returns `false` once the receiving side is gone, so the source can stop early.
    pub fn push(&self, samples: Vec<f32>) -> bool {
        self.tx.send(AudioEvent::Samples(samples)).is_ok()
    }

    pub fn end(&self) {
        let _ = self.tx.send(AudioEvent::Ended);
    }

    pub fn fail(&self, error: AudioError) {
        let _ = self.tx.send(AudioEvent::Failed(error));
    }
}

/// A producer of audio for one dictation. Implementations: the microphone
/// (phase 1) and [`super::WavFileSource`].
pub trait AudioSource: Send {
    /// Short description for logs and diagnostics, e.g. a device name or file path.
    fn describe(&self) -> String;

    /// Starts delivering audio to `sink` from a source-owned thread or callback
    /// and returns the format of the samples it will push.
    fn start(&mut self, sink: AudioSink) -> Result<AudioFormat, AudioError>;

    /// Stops delivery. Idempotent; nothing is pushed after it returns.
    fn stop(&mut self);
}
