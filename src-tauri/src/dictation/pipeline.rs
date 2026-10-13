//! After recording: trim silence, transcribe, edit, insert.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::state::{Delivered, Outcome, Processed};
use crate::audio::{AudioClip, level};
use crate::edit::Editor;
use crate::notify::Notice;
use crate::platform::{AppIdentity, InsertOutcome, InsertText, TextInserter};
use crate::stt::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};

#[derive(Clone)]
pub struct Pipeline {
    /// The engine the settings choose.
    pub engine: Arc<dyn SttEngine>,
    /// Local Whisper, used when a cloud engine fails.
    pub fallback: Option<Arc<dyn SttEngine>>,
    pub editor: Arc<Editor>,
    pub inserter: Arc<dyn TextInserter>,
    /// Languages the user speaks; the dictionary is added per dictation.
    pub languages: Vec<String>,
    pub clipboard_hold: Duration,
}

impl Pipeline {
    pub async fn transcribe_and_insert(
        &self,
        clip: AudioClip,
        app: Option<AppIdentity>,
        paste_delay: Duration,
    ) -> Processed {
        let options = SttOptions {
            languages: self.languages.clone(),
            vocabulary: self.editor.vocabulary().await,
        };
        let mut notices = Vec::new();
        let transcript = match transcribe(self.engine.as_ref(), &clip, &options).await {
            Err(SttError::NoSpeech) => Err(SttError::NoSpeech),
            Err(error) => match self.fall_back(&clip, &options, &error).await {
                Some(Ok(transcript)) => {
                    notices.push(Notice::warning(format!(
                        "{} Used Whisper on this Mac instead.",
                        cloud_failure(&error)
                    )));
                    Ok(transcript)
                }
                Some(Err(_)) | None => Err(error),
            },
            ok => ok,
        };
        let raw = match transcript {
            Ok(transcript) => transcript.text,
            Err(SttError::NoSpeech) => return Outcome::NoSpeech.into(),
            Err(SttError::Unavailable(message)) => {
                return Outcome::SttFailed {
                    clip,
                    message,
                    unavailable: true,
                }
                .into();
            }
            Err(error) => {
                return Outcome::SttFailed {
                    clip,
                    message: error.to_string(),
                    unavailable: false,
                }
                .into();
            }
        };

        let edited = self.editor.edit(&raw, app.as_ref()).await;
        notices.extend(edited.notice);
        if edited.text.plain.trim().is_empty() {
            return Processed {
                outcome: Outcome::NothingToInsert,
                notices,
            };
        }
        let delivered = Delivered {
            text: InsertText {
                plain: edited.text.plain,
                html: edited.text.html,
            },
            formatted: edited.formatted,
        };
        Processed {
            outcome: self.insert(delivered, paste_delay).await,
            notices,
        }
    }

    /// Transcribes with local Whisper when the chosen cloud engine failed.
    async fn fall_back(
        &self,
        clip: &AudioClip,
        options: &SttOptions,
        error: &SttError,
    ) -> Option<Result<Transcript, SttError>> {
        let fallback = self.fallback.as_ref()?;
        if self.engine.id() == SttEngineId::LocalWhisper || matches!(error, SttError::Cancelled) {
            return None;
        }
        log::warn!(
            "{} transcription failed ({error}); trying Whisper",
            self.engine.id().label()
        );
        Some(transcribe(fallback.as_ref(), clip, options).await)
    }

    pub async fn insert(&self, delivered: Delivered, paste_delay: Duration) -> Outcome {
        if !paste_delay.is_zero() {
            tokio::time::sleep(paste_delay).await;
        }
        let inserter = Arc::clone(&self.inserter);
        let hold = self.clipboard_hold;
        let text = delivered.text.clone();
        let result = tokio::task::spawn_blocking(move || inserter.insert(&text, hold)).await;
        match result {
            Ok(Ok(InsertOutcome::Pasted)) => Outcome::Inserted(delivered),
            Ok(Ok(InsertOutcome::Copied(reason))) => Outcome::Copied(delivered, reason),
            Ok(Err(error)) => Outcome::InsertFailed(delivered, error.to_string()),
            Err(error) => Outcome::InsertFailed(delivered, error.to_string()),
        }
    }
}

fn cloud_failure(error: &SttError) -> String {
    match error {
        SttError::Unavailable(message) => message.clone(),
        error => format!("Cloud transcription failed ({error})."),
    }
}

/// Trims silence (no signal means no STT call at all) and transcribes.
pub async fn transcribe(
    engine: &dyn SttEngine,
    clip: &AudioClip,
    options: &SttOptions,
) -> Result<Transcript, SttError> {
    let speech = level::trim_silence(clip).ok_or(SttError::NoSpeech)?;
    let started = Instant::now();
    let transcript = engine.transcribe(&speech, options).await?;
    log::info!(
        "{} transcribed {:.2} s of speech (from {:.2} s) in {} ms, language {:?}",
        engine.id().label(),
        speech.duration().as_secs_f32(),
        clip.duration().as_secs_f32(),
        started.elapsed().as_millis(),
        transcript.language
    );
    Ok(transcript)
}
