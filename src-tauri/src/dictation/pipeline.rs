//! After recording: trim silence, transcribe, edit, insert.

use std::sync::Arc;
use std::time::{Duration, Instant};

use handy_keys::Modifiers;

use super::learn::LearnTarget;
use super::state::{Delivered, Outcome, Processed};
use crate::audio::{AudioClip, level};
use crate::context::{AppKind, DictationContext, PendingContext};
use crate::edit::Editor;
use crate::lang;
use crate::notify::Notice;
use crate::settings::SettingsStore;

use crate::platform::{
    FieldKind, InsertOptions, InsertOutcome, InsertText, PasteSplit, SyntheticKey, TextInserter,
};
use crate::stt::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};

/// Lets the app take the paste before Enter arrives (CORE-16).
const ENTER_AFTER_PASTE: Duration = Duration::from_millis(120);

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
    /// The languages to transcribe in (LANG-01..05), read per dictation.
    pub settings: Arc<SettingsStore>,
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
        let language_check = (!self.settings.get().languages.auto_detect).then(|| clip.clone());
        let (raw, language, mut notices) = match self.transcribe_clip(clip, &context.ready()).await
        {
            Ok(transcribed) => transcribed,
            Err(outcome) => return outcome.into(),
        };
        let mut context = context.ready();
        context.language = language;
        let edited = self.editor.edit(&raw, &context).await;
        notices.extend(edited.notice);
        if edited.text.plain.trim().is_empty() {
            let outcome = if edited.press_enter {
                self.press_enter(paste_delay).await
            } else {
                Outcome::NothingToInsert
            };
            return Processed {
                outcome,
                notices,
                learn: None,
                language_check: None,
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
        let mut outcome = self.insert(delivered, options, paste_delay).await;
        if edited.press_enter && matches!(outcome, Outcome::Inserted(_)) {
            // After the app has taken the paste (CORE-16).
            if let Outcome::InsertFailed(_, message) = self.press_enter(ENTER_AFTER_PASTE).await {
                outcome = match outcome {
                    Outcome::Inserted(delivered) => Outcome::InsertFailed(delivered, message),
                    other => other,
                };
            }
        }
        // A plain dictation in a text field the user may now correct (DIC-07).
        let learn = match (&outcome, context.readable_field()) {
            (Outcome::Inserted(delivered), Some(field))
                if field.kind == FieldKind::Text && delivered.text.html.is_none() =>
            {
                Some(LearnTarget {
                    field_id: field.field_id,
                    inserted: delivered.text.plain.trim().to_owned(),
                })
            }
            _ => None,
        };
        Processed {
            outcome,
            notices,
            learn,
            language_check,
        }
    }

    /// Transcribes with the chosen engine, falling back to local Whisper when
    /// a cloud engine fails; the notices say so.
    pub async fn transcribe_clip(
        &self,
        clip: AudioClip,
        context: &DictationContext,
    ) -> Result<(String, Option<String>, Vec<Notice>), Outcome> {
        let languages = self.settings.get().languages;
        let options = SttOptions {
            languages: lang::stt_languages(&languages),
            vocabulary: self.editor.vocabulary(context).await,
            hints: lang::whisper_hints(&languages),
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
        match transcript {
            Ok(transcript) => Ok((transcript.text, transcript.language, notices)),
            Err(SttError::NoSpeech) => Err(Outcome::NoSpeech),
            Err(SttError::Unavailable(message)) => Err(Outcome::SttFailed {
                clip,
                message,
                unavailable: true,
            }),
            Err(error) => Err(Outcome::SttFailed {
                clip,
                message: error.to_string(),
                unavailable: false,
            }),
        }
    }

    /// Presses Enter in the focused app after `delay`.
    pub async fn press_enter(&self, delay: Duration) -> Outcome {
        tokio::time::sleep(delay).await;
        let inserter = Arc::clone(&self.inserter);
        let pressed = tokio::task::spawn_blocking(move || {
            inserter.press(SyntheticKey::Enter, Modifiers::empty())
        })
        .await;
        match pressed {
            Ok(Ok(())) => Outcome::EnterPressed,
            Ok(Err(error)) => Outcome::InsertFailed(Delivered::default(), error.to_string()),
            Err(error) => Outcome::InsertFailed(Delivered::default(), error.to_string()),
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

/// Trims silence (no signal means no STT call at all), raises whispered
/// speech to a normal level (MIC-09) and transcribes.
pub async fn transcribe(
    engine: &dyn SttEngine,
    clip: &AudioClip,
    options: &SttOptions,
) -> Result<Transcript, SttError> {
    let speech = level::trim_silence(clip).ok_or(SttError::NoSpeech)?;
    let speech = level::normalize(&speech);
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
