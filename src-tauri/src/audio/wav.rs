use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use hound::{SampleFormat, WavReader};

use super::{AudioError, AudioFormat, AudioSink, AudioSource};

/// How fast a [`WavFileSource`] delivers its samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pacing {
    /// As fast as possible. For tests.
    Instant,
    /// One chunk per chunk-duration of wall time, like a live microphone. For
    /// dev builds, so the Flow Bar waveform and timing behave realistically.
    Realtime,
}

const CHUNK_DURATION: Duration = Duration::from_millis(20);

/// Plays a WAV file into the dictation pipeline in place of the microphone.
pub struct WavFileSource {
    path: PathBuf,
    pacing: Pacing,
    worker: Option<Worker>,
}

struct Worker {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

impl WavFileSource {
    pub fn new(path: impl Into<PathBuf>, pacing: Pacing) -> Self {
        Self {
            path: path.into(),
            pacing,
            worker: None,
        }
    }
}

impl AudioSource for WavFileSource {
    fn describe(&self) -> String {
        format!("WAV file {}", self.path.display())
    }

    fn start(&mut self, sink: AudioSink) -> Result<AudioFormat, AudioError> {
        if self.worker.is_some() {
            return Err(AudioError::AlreadyStarted);
        }
        let (format, samples) = read_wav(&self.path)?;
        let stop = Arc::new(AtomicBool::new(false));
        let pacing = self.pacing;
        let worker_stop = Arc::clone(&stop);
        let handle = std::thread::Builder::new()
            .name("shrimp-wav-source".into())
            .spawn(move || deliver(&samples, format, pacing, &worker_stop, &sink))
            .map_err(|err| AudioError::Device(format!("could not start WAV thread: {err}")))?;
        self.worker = Some(Worker { stop, handle });
        Ok(format)
    }

    fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            worker.stop.store(true, Ordering::Relaxed);
            let _ = worker.handle.join();
        }
    }
}

impl Drop for WavFileSource {
    fn drop(&mut self) {
        self.stop();
    }
}

fn deliver(
    samples: &[f32],
    format: AudioFormat,
    pacing: Pacing,
    stop: &AtomicBool,
    sink: &AudioSink,
) {
    let frames_per_chunk =
        (format.sample_rate as usize * CHUNK_DURATION.as_millis() as usize / 1000).max(1);
    let started = Instant::now();
    for (index, chunk) in samples
        .chunks(frames_per_chunk * usize::from(format.channels))
        .enumerate()
    {
        if pacing == Pacing::Realtime {
            // A microphone hands over a chunk once it has been fully captured.
            let due = CHUNK_DURATION * (index as u32 + 1);
            if let Some(wait) = due.checked_sub(started.elapsed()) {
                std::thread::sleep(wait);
            }
        }
        if stop.load(Ordering::Relaxed) || !sink.push(chunk.to_vec()) {
            return;
        }
    }
    sink.end();
}

/// Reads a whole WAV file as interleaved f32 samples in `[-1.0, 1.0]`.
pub fn read_wav(path: &Path) -> Result<(AudioFormat, Vec<f32>), AudioError> {
    let wav_error = |source| AudioError::Wav {
        path: path.to_path_buf(),
        source,
    };
    let reader = WavReader::open(path).map_err(wav_error)?;
    let spec = reader.spec();
    if spec.channels == 0 || spec.sample_rate == 0 {
        return Err(AudioError::UnsupportedFormat(format!(
            "{} channels at {} Hz",
            spec.channels, spec.sample_rate
        )));
    }
    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Float, 32) => reader.into_samples::<f32>().collect::<Result<Vec<_>, _>>(),
        (SampleFormat::Int, bits @ 8..=32) => {
            let scale = 1.0 / (1_i64 << (bits - 1)) as f32;
            reader
                .into_samples::<i32>()
                .map(|sample| sample.map(|value| value as f32 * scale))
                .collect()
        }
        (sample_format, bits) => {
            return Err(AudioError::UnsupportedFormat(format!(
                "{bits}-bit {sample_format:?} samples"
            )));
        }
    }
    .map_err(wav_error)?;
    let format = AudioFormat {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
    };
    Ok((format, samples))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::AudioEvent;
    use crate::audio::test_support::write_wav;

    #[test]
    fn reads_int16_wav_as_normalized_floats() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("int16.wav");
        write_wav(&path, 16_000, 1, &[0.0, 0.5, -0.5, 1.0]);

        let (format, samples) = read_wav(&path).unwrap();

        assert_eq!(
            format,
            AudioFormat {
                sample_rate: 16_000,
                channels: 1
            }
        );
        let expected = [0.0, 0.5, -0.5, i16::MAX as f32 / 32_768.0];
        for (actual, expected) in samples.iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-4, "{actual} != {expected}");
        }
    }

    #[test]
    fn reports_missing_files() {
        let error = read_wav(Path::new("/nonexistent/shrimp.wav")).unwrap_err();
        assert!(matches!(error, AudioError::Wav { .. }));
    }

    #[test]
    fn delivers_every_sample_then_ends() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tone.wav");
        let samples: Vec<f32> = (0..48_000).map(|i| (i as f32 / 48_000.0) - 0.5).collect();
        write_wav(&path, 48_000, 2, &samples);

        let mut source = WavFileSource::new(&path, Pacing::Instant);
        let (sink, events) = AudioSink::channel();
        let format = source.start(sink).unwrap();

        let mut delivered = 0;
        loop {
            match events.recv_timeout(Duration::from_secs(5)).unwrap() {
                AudioEvent::Samples(chunk) => delivered += chunk.len(),
                AudioEvent::Ended => break,
                AudioEvent::Failed(error) => panic!("source failed: {error}"),
            }
        }
        source.stop();

        assert_eq!(
            format,
            AudioFormat {
                sample_rate: 48_000,
                channels: 2
            }
        );
        assert_eq!(delivered, samples.len());
    }

    #[test]
    fn refuses_to_start_twice() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("short.wav");
        write_wav(&path, 16_000, 1, &[0.0; 160]);

        let mut source = WavFileSource::new(&path, Pacing::Realtime);
        let (sink, _events) = AudioSink::channel();
        source.start(sink.clone()).unwrap();

        assert!(matches!(
            source.start(sink),
            Err(AudioError::AlreadyStarted)
        ));
    }
}
