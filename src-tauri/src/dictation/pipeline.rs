//! After recording: trim silence, transcribe, edit, insert.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::state::{Delivered, Outcome, Processed};
use crate::audio::{AudioClip, level};
use crate::context::{AppKind, DictationContext, PendingContext};
use crate::edit::Editor;
use crate::notify::Notice;
use crate::platform::{
    FieldKind, InsertOptions, InsertOutcome, InsertText, PasteSplit, TextInserter,
};
use crate::stt::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};

/// Parts small enough that AI coding CLIs show them inline: Claude Code
/// collapses a paste over 800 characters or 2 line breaks, Codex over 1,000
/// characters (INS-09).
const TERMINAL_SPLIT: PasteSplit = PasteSplit {
    max_chars: 700,
    max_breaks: 2,
    pause: Duration::from_millis(150),
};

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
    /// Remote desktops read the clipboard late (INS-10).
    pub remote_clipboard_hold: Duration,
}

impl Pipeline {
    pub async fn transcribe_and_insert(
        &self,
        clip: AudioClip,
        mut context: PendingContext,
        paste_delay: Duration,
    ) -> Processed {
        let options = SttOptions {
            languages: self.languages.clone(),
            vocabulary: self.editor.vocabulary(&context.ready()).await,
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

        let context = context.ready();
        let edited = self.editor.edit(&raw, &context).await;
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
        let options = self.insert_options(&context);
        Processed {
            outcome: self.insert(delivered, options, paste_delay).await,
            notices,
        }
    }

    /// Terminals paste without a field check and in parts; remote desktops
    /// hold the clipboard longer (INS-09..11).
    pub fn insert_options(&self, context: &DictationContext) -> InsertOptions {
        let mut options = InsertOptions::new(self.clipboard_hold);
        match context.app_kind() {
            AppKind::RemoteDesktop => {
                options.hold = self.remote_clipboard_hold;
                options.require_field = false;
            }
            AppKind::Terminal => {
                options.require_field = false;
                options.split = Some(TERMINAL_SPLIT);
            }
            _ if context.field_kind() == FieldKind::Terminal => {
                options.split = Some(TERMINAL_SPLIT);
            }
            _ => {}
        }
        options
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

    pub async fn insert(
        &self,
        delivered: Delivered,
        options: InsertOptions,
        paste_delay: Duration,
    ) -> Outcome {
        if !paste_delay.is_zero() {
            tokio::time::sleep(paste_delay).await;
        }
        let inserter = Arc::clone(&self.inserter);
        let text = delivered.text.clone();
        let result = tokio::task::spawn_blocking(move || inserter.insert(&text, &options)).await;
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
