use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use super::{AudioClip, AudioError, AudioEvent, AudioFormat, AudioSink, AudioSource};

/// Collects the audio a source delivers between [`Recorder::start`] and
/// [`Recorder::finish`]. Dropping a recorder stops the source and discards
/// the audio (cancelled dictation).
pub struct Recorder {
    source: Box<dyn AudioSource>,
    events: Receiver<AudioEvent>,
    format: AudioFormat,
    samples: Vec<f32>,
    ended: bool,
}

impl Recorder {
    pub fn start(mut source: Box<dyn AudioSource>) -> Result<Self, AudioError> {
        let (sink, events) = AudioSink::channel();
        let format = source.start(sink)?;
        log::debug!("recording from {} ({format:?})", source.describe());
        Ok(Self {
            source,
            events,
            format,
            samples: Vec::new(),
            ended: false,
        })
    }

    pub fn format(&self) -> AudioFormat {
        self.format
    }

    /// Whether the source ran out of audio by itself.
    pub fn has_ended(&self) -> bool {
        self.ended
    }

    /// Moves delivered audio into the buffer without blocking and returns the
    /// samples this call added, e.g. for level metering.
    pub fn poll(&mut self) -> Result<&[f32], AudioError> {
        let before = self.samples.len();
        while let Ok(event) = self.events.try_recv() {
            self.apply(event)?;
        }
        Ok(&self.samples[before..])
    }

    /// Blocks until the source ends by itself or `timeout` passes, and
    /// returns whether it ended. For finite sources such as WAV files.
    pub fn wait_until_ended(&mut self, timeout: Duration) -> Result<bool, AudioError> {
        let deadline = Instant::now() + timeout;
        while !self.ended {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self.events.recv_timeout(remaining) {
                Ok(event) => self.apply(event)?,
                Err(RecvTimeoutError::Timeout) => return Ok(false),
                Err(RecvTimeoutError::Disconnected) => return Err(AudioError::Disconnected),
            }
        }
        Ok(true)
    }

    /// Stops the source, keeps everything it delivered and converts it for STT.
    pub fn finish(mut self) -> Result<AudioClip, AudioError> {
        self.source.stop();
        self.poll()?;
        AudioClip::from_interleaved(self.format, &self.samples)
    }

    fn apply(&mut self, event: AudioEvent) -> Result<(), AudioError> {
        match event {
            AudioEvent::Samples(chunk) => self.samples.extend_from_slice(&chunk),
            AudioEvent::Ended => self.ended = true,
            AudioEvent::Failed(error) => return Err(error),
        }
        Ok(())
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.source.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::test_support::write_wav;
    use crate::audio::{Pacing, STT_SAMPLE_RATE, WavFileSource};

    #[test]
    fn records_a_wav_file_into_a_16k_mono_clip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("speech.wav");
        write_wav(&path, 48_000, 2, &vec![0.25; 48_000 * 2]);

        let mut recorder =
            Recorder::start(Box::new(WavFileSource::new(&path, Pacing::Instant))).unwrap();
        assert!(recorder.wait_until_ended(Duration::from_secs(5)).unwrap());
        let clip = recorder.finish().unwrap();

        assert!(clip.samples().len().abs_diff(STT_SAMPLE_RATE as usize) <= 2);
    }

    #[test]
    fn finish_keeps_audio_captured_before_stop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("long.wav");
        write_wav(&path, 16_000, 1, &vec![0.1; 16_000 * 10]);

        let mut recorder =
            Recorder::start(Box::new(WavFileSource::new(&path, Pacing::Realtime))).unwrap();
        std::thread::sleep(Duration::from_millis(200));
        assert!(!recorder.poll().unwrap().is_empty());
        let clip = recorder.finish().unwrap();

        // Stopped early: some audio, far less than the 10 s file.
        assert!(!clip.is_empty());
        assert!(clip.duration() < Duration::from_secs(2));
    }
}
