//! Audio input for dictation.
//!
//! Every input implements [`AudioSource`]: the live microphone in normal use,
//! a WAV file in tests and dev builds. A [`Recorder`] collects what a source
//! delivers and converts it to the [`AudioClip`] format every STT engine takes
//! (16 kHz mono f32), so the rest of the pipeline never knows where audio came from.

mod clip;
mod error;
mod recorder;
mod source;
mod spec;
#[cfg(test)]
mod test_support;
mod wav;

pub use clip::{AudioClip, STT_SAMPLE_RATE};
pub use error::AudioError;
pub use recorder::Recorder;
pub use source::{AudioEvent, AudioFormat, AudioSink, AudioSource};
pub use spec::{AUDIO_SOURCE_ENV, AudioSourceSpec};
pub use wav::{Pacing, WavFileSource, read_wav};
