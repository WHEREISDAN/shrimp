//! After recording: trim silence, transcribe, edit, insert.

use std::sync::Arc;
use std::time::{Duration, Instant};

use handy_keys::Modifiers;

use super::learn::LearnTarget;
use super::state::{Delivered, Outcome, Processed};
use crate::audio::{AudioClip, level};
use crate::context::{AppKind, DictationContext, PendingContext};
use crate::edit::tips::{self, Formatting};
use crate::edit::{CleanupLevel, Editor};
use crate::history::{Dictation, History};
use crate::insights;
use crate::lang;
use crate::notify::{Notice, NoticeCategory};
use crate::settings::SettingsStore;
use crate::storage::{HistoryStatus, now_ms};

use crate::platform::{
    AppIdentity, FieldKind, InsertOptions, InsertOutcome, InsertText, PasteSplit, SyntheticKey,
    TextInserter,
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
    /// Where finished dictations are stored (HIS-01).
    pub history: Option<Arc<History>>,
}

impl Pipeline {
    pub async fn transcribe_and_insert(
        &self,
        clip: AudioClip,
        mut context: PendingContext,
        paste_delay: Duration,
    ) -> Processed {
        let settings = self.settings.get().effective();
        let language_check = (!settings.languages.auto_detect).then(|| clip.clone());
        let mut record = self.record(&clip, context.app().cloned());
        let started = Instant::now();
        let transcribed = self.transcribe_clip(clip, &context.ready()).await;
        record.stt_ms = Some(started.elapsed().as_millis() as i64);
        let (raw, language, mut notices) = match transcribed {
            Ok(transcribed) => transcribed,
            Err(outcome) => {
                if let Outcome::SttFailed { message, .. } = &outcome {
                    record.error = Some(message.clone());
                }
                return Processed {
                    record: Some(record),
                    ..outcome.into()
                };
            }
        };
        let mut context = context.ready();
        context.language = language.clone();
        let edited = self.editor.edit(&raw, &context).await;
        notices.extend(edited.notice.clone());
        record.raw = raw;
        record.text = edited.text.plain.trim().to_owned();
        record.formatted = edited.formatted;
        record.language = language;
        record.llm_ms = edited.llm_ms.map(|ms| ms as i64);
        record.llm_provider = (settings.ai.cleanup != CleanupLevel::None && edited.formatted)
            .then_some(settings.ai.provider);
        record.dictionary_fixes = edited.dictionary_fixes as i64;
        if edited.text.plain.trim().is_empty() {
            let outcome = if edited.press_enter {
                self.press_enter(paste_delay).await
            } else {
                Outcome::NothingToInsert
            };
            return Processed {
                notices,
                ..outcome.into()
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
        if matches!(outcome, Outcome::Inserted(_))
            && let Some(tip) = self.formatting_tip(edited.formatting)
        {
            notices.push(tip);
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
            notices,
            learn,
            language_check,
            record: Some(record),
            ..outcome.into()
        }
    }

    /// The formatting tip this dictation earns (FMT-13), counted when shown.
    fn formatting_tip(&self, formatting: Formatting) -> Option<Notice> {
        let settings = self.settings.get();
        if !NoticeCategory::Announcement.allowed(&settings.notifications) {
            return None;
        }
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let (tip, explanation, shown) =
            tips::next(formatting, &settings.notices.formatting_tips, &today)?;
        if let Err(error) = self
            .settings
            .update(&serde_json::json!({"notices": {"formattingTips": shown}}))
        {
            log::warn!("could not count the tip: {error}");
        }
        Some(tips::notice(tip, explanation))
    }

    /// What History will store for `clip`, before it is transcribed.
    fn record(&self, clip: &AudioClip, app: Option<AppIdentity>) -> Dictation {
        let speech = level::trim_silence(clip).map_or(Duration::ZERO, |speech| speech.duration());
        Dictation {
            created_at: now_ms(),
            app,
            stt_engine: Some(self.engine.id()),
            audio_ms: clip.duration().as_millis() as i64,
            speech_ms: speech.as_millis() as i64,
            clip: Some(clip.clone()),
            ..Dictation::default()
        }
    }

    /// Stores a finished dictation in History and deletes its journal
    /// (HIS-01, HIS-06). Command Mode and transforms are not stored.
    pub async fn store(&self, processed: &mut Processed) {
        let journal = processed.journal.take();
        let (Some(history), Some(record)) = (self.history.clone(), processed.record.take()) else {
            if let Some(journal) = journal {
                journal.delete();
            }
            return;
        };
        let status = match &processed.outcome {
            Outcome::Inserted(_) => Some(HistoryStatus::Inserted),
            Outcome::Copied(..) | Outcome::InsertFailed(..) => Some(HistoryStatus::Copied),
            Outcome::SttFailed { .. } => Some(HistoryStatus::Failed),
            Outcome::NoSpeech => Some(HistoryStatus::Empty),
            _ => None,
        };
        let stored = tokio::task::spawn_blocking(move || {
            let stored = status.map(|status| history.record(record, status));
            if let Some(journal) = journal {
                journal.delete();
            }
            stored
        })
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
        processed.history_id = stored.id;
        processed
            .notices
            .extend(stored.milestone.map(insights::notice));
    }

    /// History's Retry and Recover: transcribes an entry's audio again and
    /// updates the entry; nothing is pasted (HIS-05, HIS-06).
    pub async fn retry(&self, id: i64) -> Processed {
        const FAILED: &str = "Retry failed. Please try again.";
        let Some(history) = self.history.clone() else {
            return Outcome::RetryFailed(FAILED.into()).into();
        };
        let loading = Arc::clone(&history);
        let clip = match tokio::task::spawn_blocking(move || loading.retry_clip(id)).await {
            Ok(Ok(clip)) => clip,
            Ok(Err(error)) => return Outcome::RetryFailed(error.to_string()).into(),
            Err(_) => return Outcome::RetryFailed(FAILED.into()).into(),
        };
        let mut record = Dictation {
            replaces: Some(id),
            clip: None,
            ..self.record(&clip, None)
        };
        let started = Instant::now();
        let transcribed = self
            .transcribe_clip(clip, &DictationContext::default())
            .await;
        record.stt_ms = Some(started.elapsed().as_millis() as i64);
        let (outcome, status, mut notices) = match transcribed {
            Ok((raw, language, notices)) => {
                let context = DictationContext {
                    language: language.clone(),
                    ..DictationContext::default()
                };
                let edited = self.editor.edit(&raw, &context).await;
                record.raw = raw;
                record.text = edited.text.plain.trim().to_owned();
                record.formatted = edited.formatted;
                record.language = language;
                record.dictionary_fixes = edited.dictionary_fixes as i64;
                let mut notices = notices;
                notices.extend(edited.notice);
                let status = if record.text.is_empty() {
                    HistoryStatus::Empty
                } else {
                    HistoryStatus::Transcribed
                };
                (Outcome::Retried, status, notices)
            }
            Err(Outcome::NoSpeech) => (Outcome::NoSpeech, HistoryStatus::Empty, Vec::new()),
            Err(Outcome::SttFailed { message, .. }) => {
                record.error = Some(message);
                (
                    Outcome::RetryFailed(FAILED.into()),
                    HistoryStatus::Failed,
                    Vec::new(),
                )
            }
            Err(_) => (
                Outcome::RetryFailed(FAILED.into()),
                HistoryStatus::Failed,
                Vec::new(),
            ),
        };
        match tokio::task::spawn_blocking(move || history.record(record, status)).await {
            Ok(stored) if stored.id.is_some() => {
                notices.extend(stored.milestone.map(insights::notice));
            }
            _ => notices.push(Notice::error(FAILED)),
        }
        Processed {
            notices,
            ..outcome.into()
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
