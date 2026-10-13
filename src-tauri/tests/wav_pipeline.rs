//! The STT pipeline fed from a WAV file instead of the microphone: the same
//! path dev builds use with SHRIMP_AUDIO_SOURCE=wav:<path>.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use shrimp_lib::audio::{AudioClip, Pacing, Recorder, STT_SAMPLE_RATE, WavFileSource};
use shrimp_lib::stt::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/audio")
        .join(name)
}

/// Records what it was given instead of transcribing.
#[derive(Default)]
struct CapturingEngine {
    received: Mutex<Option<AudioClip>>,
}

#[async_trait]
impl SttEngine for CapturingEngine {
    fn id(&self) -> SttEngineId {
        SttEngineId::LocalWhisper
    }

    async fn transcribe(
        &self,
        clip: &AudioClip,
        _options: &SttOptions,
    ) -> Result<Transcript, SttError> {
        *self.received.lock().unwrap() = Some(clip.clone());
        Ok(Transcript {
            text: "captured".into(),
            language: Some("en".into()),
        })
    }
}

#[tokio::test]
async fn wav_fixture_reaches_the_engine_as_16k_mono() {
    let source = WavFileSource::new(fixture("hello-shrimp.wav"), Pacing::Instant);
    let mut recorder = Recorder::start(Box::new(source)).unwrap();
    assert!(recorder.wait_until_ended(Duration::from_secs(10)).unwrap());
    let clip = recorder.finish().unwrap();

    let engine = CapturingEngine::default();
    let transcript = engine
        .transcribe(&clip, &SttOptions::default())
        .await
        .unwrap();

    assert_eq!(transcript.text, "captured");
    let received = engine.received.lock().unwrap().take().unwrap();
    // The fixture is 3.69 s of 16 kHz mono speech generated with `say`.
    assert_eq!(received.samples().len(), 59_020);
    assert!((received.duration().as_secs_f64() - 3.689).abs() < 0.01);
    assert!(
        received.samples().iter().any(|sample| sample.abs() > 0.1),
        "fixture should contain speech"
    );
    assert_eq!(STT_SAMPLE_RATE, 16_000);
}
