use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use super::{AudioClip, AudioError, AudioEvent, AudioFormat, AudioSink, AudioSource};

/// Collects the audio a source delivers between [`Recorder::start`] and
/// [`Recorder::finish`]. A source may change format midway (a microphone
/// fallback, MIC-03); each run of one format is kept and converted on its
/// own. Dropping a recorder stops the source and discards the audio
/// (cancelled dictation).
pub struct Recorder {
    source: Box<dyn AudioSource>,
    events: Receiver<AudioEvent>,
    segments: Vec<Segment>,
    ended: bool,
}

struct Segment {
    format: AudioFormat,
    samples: Vec<f32>,
}

impl Recorder {
    pub fn start(mut source: Box<dyn AudioSource>) -> Result<Self, AudioError> {
        let (sink, events) = AudioSink::channel();
        let format = source.start(sink)?;
        log::debug!("recording from {} ({format:?})", source.describe());
        Ok(Self {
            source,
            events,
            segments: format
                .map(|format| Segment {
                    format,
                    samples: Vec::new(),
                })
                .into_iter()
                .collect(),
            ended: false,
        })
    }

    /// The format of the samples arriving now, once the source has opened.
    pub fn format(&self) -> Option<AudioFormat> {
        self.segments.last().map(|segment| segment.format)
    }

    /// Whether the source has opened (its format is known).
    pub fn is_open(&self) -> bool {
        !self.segments.is_empty()
    }

    /// Whether the source ran out of audio by itself.
    pub fn has_ended(&self) -> bool {
        self.ended
    }

    /// Moves delivered audio into the buffer without blocking and returns the
    /// samples of the current format this call added, e.g. for level metering.
    pub fn poll(&mut self) -> Result<&[f32], AudioError> {
        let count = self.segments.len();
        let before = self
            .segments
            .last()
            .map_or(0, |segment| segment.samples.len());
        while let Ok(event) = self.events.try_recv() {
            self.apply(event)?;
        }
        let Some(last) = self.segments.last() else {
            return Ok(&[]);
        };
        let from = if self.segments.len() == count {
            before
        } else {
            0
        };
        Ok(&last.samples[from..])
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
        // Audio delivered before a failure is kept.
        while let Ok(event) = self.events.try_recv() {
            if self.apply(event).is_err() {
                break;
            }
        }
        let clips = self
            .segments
            .iter()
            .filter(|segment| !segment.samples.is_empty())
            .map(|segment| AudioClip::from_interleaved(segment.format, &segment.samples))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AudioClip::concat(&clips))
    }

    fn apply(&mut self, event: AudioEvent) -> Result<(), AudioError> {
        match event {
            AudioEvent::Format(format) => self.segments.push(Segment {
                format,
                samples: Vec::new(),
            }),
            AudioEvent::Samples(chunk) => match self.segments.last_mut() {
                Some(segment) => segment.samples.extend_from_slice(&chunk),
                None => log::warn!("audio arrived before its format; dropped"),
            },
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

    /// Sends two formats, the second after a device switch.
    struct Switching;

    impl AudioSource for Switching {
        fn describe(&self) -> String {
            "switching".into()
        }

        fn start(&mut self, sink: AudioSink) -> Result<Option<AudioFormat>, AudioError> {
            sink.format(AudioFormat {
                sample_rate: 48_000,
                channels: 2,
            });
            sink.push(vec![0.2; 48_000 * 2]);
            sink.format(AudioFormat {
                sample_rate: 16_000,
                channels: 1,
            });
            sink.push(vec![0.2; 16_000]);
            sink.end();
            Ok(None)
        }

        fn stop(&mut self) {}
    }

    #[test]
    fn a_format_change_keeps_both_runs_of_audio() {
        let mut recorder = Recorder::start(Box::new(Switching)).unwrap();
        assert!(!recorder.is_open() || recorder.format().is_some());
        assert!(recorder.wait_until_ended(Duration::from_secs(1)).unwrap());
        assert_eq!(recorder.format().unwrap().sample_rate, 16_000);
        let clip = recorder.finish().unwrap();
        let seconds = clip.duration().as_secs_f32();
        assert!((1.95..2.05).contains(&seconds), "{seconds} s");
    }
}
