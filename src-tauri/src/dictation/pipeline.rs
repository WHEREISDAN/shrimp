//! After recording: trim silence, transcribe, insert.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::state::Outcome;
use crate::audio::{AudioClip, level};
use crate::platform::{InsertOutcome, TextInserter};
use crate::stt::{SttEngine, SttError, SttOptions};

#[derive(Clone)]
pub struct Pipeline {
    pub engine: Arc<dyn SttEngine>,
    pub inserter: Arc<dyn TextInserter>,
    pub options: SttOptions,
    pub clipboard_hold: Duration,
}

impl Pipeline {
    pub async fn transcribe_and_insert(&self, clip: AudioClip, paste_delay: Duration) -> Outcome {
        match transcribe(self.engine.as_ref(), &clip, &self.options).await {
            Ok(text) => self.insert(text, paste_delay).await,
            Err(SttError::NoSpeech) => Outcome::NoSpeech,
            Err(SttError::Unavailable(message)) => Outcome::SttFailed {
                clip,
                message,
                unavailable: true,
            },
            Err(error) => Outcome::SttFailed {
                clip,
                message: error.to_string(),
                unavailable: false,
            },
        }
    }

    pub async fn insert(&self, text: String, paste_delay: Duration) -> Outcome {
        if !paste_delay.is_zero() {
            tokio::time::sleep(paste_delay).await;
        }
        let inserter = Arc::clone(&self.inserter);
        let hold = self.clipboard_hold;
        let pasted = text.clone();
        let result = tokio::task::spawn_blocking(move || inserter.insert(&pasted, hold)).await;
        match result {
            Ok(Ok(InsertOutcome::Pasted)) => Outcome::Inserted(text),
            Ok(Ok(InsertOutcome::Copied(reason))) => Outcome::Copied(text, reason),
            Ok(Err(error)) => Outcome::InsertFailed(text, error.to_string()),
            Err(error) => Outcome::InsertFailed(text, error.to_string()),
        }
    }
}

/// Trims silence (no signal means no STT call at all) and transcribes.
pub async fn transcribe(
    engine: &dyn SttEngine,
    clip: &AudioClip,
    options: &SttOptions,
) -> Result<String, SttError> {
    let speech = level::trim_silence(clip).ok_or(SttError::NoSpeech)?;
    let started = Instant::now();
    let transcript = engine.transcribe(&speech, options).await?;
    log::info!(
        "transcribed {:.2} s of speech (from {:.2} s) in {} ms, language {:?}",
        speech.duration().as_secs_f32(),
        clip.duration().as_secs_f32(),
        started.elapsed().as_millis(),
        transcript.language
    );
    Ok(transcript.text)
}
