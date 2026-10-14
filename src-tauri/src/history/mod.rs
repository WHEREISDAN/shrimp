//! Transcript history (HIS-01..10): every dictation with its recording for
//! 14 days, retries, crash recovery, cancelled dictations, and what
//! `retention` deletes and backs up. Usage stats are counted here too, apart
//! from the transcripts, so they survive deletion (STA-05).

pub mod fixes;
pub mod journal;
mod retention;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, Local, TimeZone, Timelike};
use serde::Serialize;

use crate::audio::{AudioClip, read_wav, wav_encode};
use crate::insights::usage::UsageCategory;
use crate::insights::{self, Milestone};
use crate::llm::LlmProviderId;
use crate::platform::AppIdentity;
use crate::settings::{HistoryStorage, SettingsStore};
use crate::storage::{
    Database, HistoryEntry, HistoryStatus, NewEntry, Retranscribed, StorageError, history, now_ms,
    stats,
};
use crate::stt::SttEngineId;
use crate::t;
use journal::Journal;

/// Recordings are kept this long for Retry and Play audio (HIS-04).
pub const AUDIO_DAYS: i64 = 14;
/// Failed or empty entries need this much audio to be retried (HIS-05).
pub const MIN_RETRY_MS: i64 = 5_000;
/// Cancelled audio shorter than this is not stored (HIS-10, Wispr Flow's
/// half second, article 4363488655).
pub const MIN_CANCELLED_AUDIO: Duration = Duration::from_millis(500);
const DAY_MS: i64 = 86_400_000;
const LIST_LIMIT: usize = 1_000;

/// What a row offers for its audio. Mirrors `RetryKind` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RetryKind {
    /// A failed or empty transcription (HIS-05).
    Retry,
    /// A recording interrupted by a quit or crash (HIS-06), or cancelled (HIS-10).
    Recover,
}

/// A row of Home's history. Mirrors `HistoryItem` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    #[serde(flatten)]
    pub entry: HistoryEntry,
    pub has_audio: bool,
    pub retry: Option<RetryKind>,
}

/// Mirrors `HistoryList` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryList {
    pub entries: Vec<HistoryItem>,
    /// "Never store data locally" is on.
    pub disabled: bool,
}

/// Why a history action couldn't run, in Wispr Flow's words.
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("{}", t!("error.history.retryAudioGone"))]
    RetryAudioGone,
    #[error("{}", t!("error.history.playAudioGone"))]
    PlayAudioGone,
    #[error("{}", t!("error.history.downloadAudioGone"))]
    DownloadAudioGone,
    #[error("{}", t!("error.history.notRetryable"))]
    NotRetryable,
    #[error("{}", t!("error.history.notFound"))]
    NotFound,
    #[error("{0}")]
    Storage(StorageError),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl From<StorageError> for HistoryError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::NotFound => Self::NotFound,
            error => Self::Storage(error),
        }
    }
}

/// What storing a dictation did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stored {
    /// The History entry, unless nothing is stored.
    pub id: Option<i64>,
    /// A milestone the dictation's words reached.
    pub milestone: Option<Milestone>,
}

/// A finished dictation, as the pipeline reports it.
#[derive(Debug, Clone, Default)]
pub struct Dictation {
    pub created_at: i64,
    pub app: Option<AppIdentity>,
    pub raw: String,
    pub text: String,
    pub formatted: bool,
    pub language: Option<String>,
    pub stt_engine: Option<SttEngineId>,
    pub llm_provider: Option<LlmProviderId>,
    pub audio_ms: i64,
    /// Without leading and trailing silence, for speed (STA-01).
    pub speech_ms: i64,
    pub stt_ms: Option<i64>,
    pub llm_ms: Option<i64>,
    /// Dictionary corrections and snippets applied.
    pub dictionary_fixes: i64,
    pub clip: Option<AudioClip>,
    pub error: Option<String>,
    /// The entry this transcribes again (paste last of a failed dictation).
    pub replaces: Option<i64>,
    /// Where it was dictated, for Insights (STA-06); unknown for retries.
    pub usage: Option<UsageCategory>,
}

impl Dictation {
    pub fn words(&self) -> i64 {
        self.text.split_whitespace().count() as i64
    }
}

pub struct History {
    database: Arc<Database>,
    recordings: PathBuf,
    backups: PathBuf,
    settings: Arc<SettingsStore>,
    changed: Box<dyn Fn() + Send + Sync>,
}

impl History {
    /// `data_dir` holds `recordings/` and `backups/`; `changed` is called
    /// whenever the list changes.
    pub fn new(
        database: Arc<Database>,
        data_dir: &Path,
        settings: Arc<SettingsStore>,
        changed: Box<dyn Fn() + Send + Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database,
            recordings: data_dir.join("recordings"),
            backups: data_dir.join("backups"),
            settings,
            changed,
        })
    }

    fn storage(&self) -> HistoryStorage {
        self.settings.get().privacy.history
    }

    /// Whether new dictations are stored ("Never store" is off).
    pub fn stores_dictations(&self) -> bool {
        self.storage() != HistoryStorage::Never
    }

    /// Where the recording in progress is saved as it arrives (HIS-06);
    /// nothing is saved with "Never store".
    pub fn start_journal(&self) -> Option<Journal> {
        self.stores_dictations()
            .then(|| Journal::new(&self.recordings, now_ms()))
    }

    /// Stores a dictation and counts its words. Blocking: call from the
    /// blocking pool.
    pub fn record(&self, dictation: Dictation, status: HistoryStatus) -> Stored {
        let milestone = status.has_text().then(|| self.count(&dictation)).flatten();
        Stored {
            id: self.store(dictation, status),
            milestone,
        }
    }

    /// Stores a cancelled recording to recover later (HIS-10); its words
    /// count once it is transcribed. Blocking: call from the blocking pool.
    pub fn record_cancelled(&self, clip: &AudioClip, app: Option<AppIdentity>) -> Option<i64> {
        let dictation = Dictation {
            created_at: now_ms(),
            app,
            audio_ms: clip.duration().as_millis() as i64,
            clip: Some(clip.clone()),
            ..Dictation::default()
        };
        self.record(dictation, HistoryStatus::Cancelled).id
    }

    fn store(&self, dictation: Dictation, status: HistoryStatus) -> Option<i64> {
        if self.storage() == HistoryStorage::Never {
            return None;
        }
        let stored = match dictation.replaces {
            Some(id) => self
                .retranscribed(id, &dictation, status)
                .map(|entry| entry.id),
            None => self.insert(&dictation, status),
        };
        let id = match stored {
            Ok(id) => id,
            Err(error) => {
                log::warn!("could not store the dictation in History: {error}");
                return None;
            }
        };
        (self.changed)();
        Some(id)
    }

    fn insert(&self, dictation: &Dictation, status: HistoryStatus) -> Result<i64, HistoryError> {
        let new = NewEntry {
            created_at: dictation.created_at,
            mode: "dictation",
            status: Some(status),
            app_id: dictation.app.as_ref().map(|app| app.bundle_id.clone()),
            app_name: dictation.app.as_ref().map(|app| app.name.clone()),
            raw_text: dictation.raw.clone(),
            final_text: dictation.text.clone(),
            formatted: dictation.formatted,
            language: dictation.language.clone(),
            stt_engine: dictation.stt_engine.map(|id| id.label().to_owned()),
            llm_provider: dictation.llm_provider.map(|id| id.label().to_owned()),
            audio_ms: dictation.audio_ms,
            stt_ms: dictation.stt_ms,
            llm_ms: dictation.llm_ms,
            word_count: dictation.words(),
            error: dictation.error.clone(),
        };
        let audio = dictation
            .clip
            .as_ref()
            .and_then(|clip| wav_encode::wav_bytes(clip).ok());
        let recordings = self.recordings.clone();
        // One lock, so the entry is never listed without its recording.
        let id = self.database.with(move |db| {
            let id = history::insert(db, &new)?.id;
            if let Some(bytes) = audio {
                let name = format!("{id}.wav");
                let saved = std::fs::create_dir_all(&recordings)
                    .and_then(|()| std::fs::write(recordings.join(&name), bytes));
                match saved {
                    Ok(()) => history::set_audio_path(db, id, Some(&name))?,
                    Err(error) => log::warn!("could not save the recording: {error}"),
                }
            }
            Ok(id)
        })?;
        Ok(id)
    }

    fn retranscribed(
        &self,
        id: i64,
        dictation: &Dictation,
        status: HistoryStatus,
    ) -> Result<HistoryEntry, HistoryError> {
        let result = Retranscribed {
            status,
            raw_text: dictation.raw.clone(),
            final_text: dictation.text.clone(),
            formatted: dictation.formatted,
            language: dictation.language.clone(),
            word_count: dictation.words(),
            error: dictation.error.clone(),
        };
        Ok(self
            .database
            .with(|db| history::update_transcript(db, id, &result))?)
    }

    /// Counts the words; returns a milestone they reached.
    fn count(&self, dictation: &Dictation) -> Option<Milestone> {
        let at = local(dictation.created_at);
        let counted = stats::Counted {
            created_at: dictation.created_at,
            day: at.format("%Y-%m-%d").to_string(),
            weekday: at.weekday().num_days_from_sunday(),
            hour: at.hour(),
            app_id: dictation.app.as_ref().map(|app| app.bundle_id.clone()),
            app_name: dictation.app.as_ref().map(|app| app.name.clone()),
            words: dictation.words(),
            audio_ms: dictation.audio_ms,
            speech_ms: dictation.speech_ms,
            fixes: fixes::words_corrected(&dictation.raw, &dictation.text),
            dictionary_fixes: dictation.dictionary_fixes,
            usage: dictation.usage.map(UsageCategory::key),
        };
        let counted = self.database.with(|db| {
            let before = stats::total_words(db)?;
            stats::record(db, &counted)?;
            Ok((before, stats::total_words(db)?))
        });
        match counted {
            Ok((before, after)) => insights::milestone(before, after),
            Err(error) => {
                log::warn!("could not count the dictation: {error}");
                None
            }
        }
    }

    /// Newest first, matching `query` (HIS-01, HIS-02).
    pub fn list(&self, query: &str) -> Result<HistoryList, HistoryError> {
        if self.storage() == HistoryStorage::Never {
            return Ok(HistoryList {
                entries: Vec::new(),
                disabled: true,
            });
        }
        let now = now_ms();
        let entries = self
            .database
            .with(|db| history::list(db, query, LIST_LIMIT))?
            .into_iter()
            .map(|entry| item(entry, now))
            .collect();
        Ok(HistoryList {
            entries,
            disabled: false,
        })
    }

    pub fn get(&self, id: i64) -> Result<HistoryItem, HistoryError> {
        let entry = self.database.with(|db| history::get(db, id))?;
        Ok(item(entry, now_ms()))
    }

    /// Undo AI edit (`raw`) or Redo AI edit (FMT-12).
    pub fn show_raw(&self, id: i64, raw: bool) -> Result<HistoryItem, HistoryError> {
        let entry = self
            .database
            .with(|db| history::set_show_raw(db, id, raw))?;
        (self.changed)();
        Ok(item(entry, now_ms()))
    }

    pub fn delete(&self, id: i64) -> Result<(), HistoryError> {
        let audio = self.database.with(|db| history::delete(db, id))?;
        self.delete_recordings(audio.into_iter().collect());
        (self.changed)();
        Ok(())
    }

    /// The recording to transcribe again (HIS-05, HIS-06).
    pub fn retry_clip(&self, id: i64) -> Result<AudioClip, HistoryError> {
        let item = self.get(id)?;
        if item.retry.is_none() {
            return Err(if item.has_audio {
                HistoryError::NotRetryable
            } else {
                HistoryError::RetryAudioGone
            });
        }
        let path = self.audio_file(&item.entry, HistoryError::RetryAudioGone)?;
        let (format, samples) = read_wav(&path).map_err(|_| HistoryError::RetryAudioGone)?;
        AudioClip::from_interleaved(format, &samples).map_err(|_| HistoryError::RetryAudioGone)
    }

    /// The recording as WAV bytes, for Play audio (HIS-03).
    pub fn audio(&self, id: i64) -> Result<Vec<u8>, HistoryError> {
        let entry = self.database.with(|db| history::get(db, id))?;
        let path = self.audio_file(&entry, HistoryError::PlayAudioGone)?;
        std::fs::read(path).map_err(|_| HistoryError::PlayAudioGone)
    }

    /// Copies the recording into `folder` as "Shrimp <date> <time>.wav"
    /// (Extract audio, HIS-03) and returns its path.
    pub fn extract(&self, id: i64, folder: &Path) -> Result<PathBuf, HistoryError> {
        let entry = self.database.with(|db| history::get(db, id))?;
        let source = self.audio_file(&entry, HistoryError::DownloadAudioGone)?;
        let stamp = local(entry.created_at).format("%Y-%m-%d %H.%M.%S");
        let mut target = folder.join(format!("Shrimp {stamp}.wav"));
        let mut copy = 2;
        while target.exists() {
            target = folder.join(format!("Shrimp {stamp} ({copy}).wav"));
            copy += 1;
        }
        std::fs::copy(source, &target)?;
        Ok(target)
    }

    fn audio_file(
        &self,
        entry: &HistoryEntry,
        gone: HistoryError,
    ) -> Result<PathBuf, HistoryError> {
        entry
            .audio_path
            .as_ref()
            .map(|name| self.recordings.join(name))
            .filter(|path| path.is_file())
            .ok_or(gone)
    }

    /// Turns recordings a quit or crash left behind into "Interrupted"
    /// entries (HIS-06). Returns how many were recovered.
    pub fn recover_interrupted(&self) -> usize {
        if self.storage() == HistoryStorage::Never {
            journal::discard_all(&self.recordings);
            return 0;
        }
        let leftovers = journal::leftovers(&self.recordings);
        let mut recovered = 0;
        for (started, clip) in leftovers {
            let dictation = Dictation {
                created_at: started,
                audio_ms: clip.duration().as_millis() as i64,
                clip: Some(clip),
                ..Dictation::default()
            };
            if self.insert(&dictation, HistoryStatus::Interrupted).is_ok() {
                recovered += 1;
            }
        }
        if recovered > 0 {
            (self.changed)();
        }
        recovered
    }
}

fn item(entry: HistoryEntry, now: i64) -> HistoryItem {
    let has_audio = entry.audio_path.is_some() && now - entry.created_at < AUDIO_DAYS * DAY_MS;
    let retry = match entry.status {
        _ if !has_audio => None,
        HistoryStatus::Interrupted | HistoryStatus::Cancelled => Some(RetryKind::Recover),
        HistoryStatus::Failed | HistoryStatus::Empty if entry.audio_ms >= MIN_RETRY_MS => {
            Some(RetryKind::Retry)
        }
        _ => None,
    };
    HistoryItem {
        entry,
        has_audio,
        retry,
    }
}

/// Unix milliseconds as local time.
pub fn local(ms: i64) -> chrono::DateTime<Local> {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .unwrap_or_else(Local::now)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::settings::Settings;

    pub(super) struct Fixture {
        pub(super) history: Arc<History>,
        pub(super) settings: Arc<SettingsStore>,
        pub(super) changes: Arc<AtomicUsize>,
        pub(super) dir: tempfile::TempDir,
    }

    pub(super) fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let database = Arc::new(Database::open(&dir.path().join("shrimp.db")).unwrap());
        let settings = Arc::new(SettingsStore::in_memory(Settings::default()));
        let changes = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&changes);
        let history = History::new(
            database,
            dir.path(),
            Arc::clone(&settings),
            Box::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            }),
        );
        Fixture {
            history,
            settings,
            changes,
            dir,
        }
    }

    pub(super) fn dictation(text: &str, seconds: usize) -> Dictation {
        Dictation {
            created_at: now_ms(),
            raw: format!("um {text}"),
            text: text.into(),
            formatted: true,
            audio_ms: seconds as i64 * 1_000,
            speech_ms: seconds as i64 * 900,
            clip: Some(AudioClip::from_mono_16k(vec![0.1; 16_000 * seconds])),
            app: Some(AppIdentity {
                bundle_id: "com.apple.TextEdit".into(),
                name: "TextEdit".into(),
            }),
            ..Dictation::default()
        }
    }

    pub(super) fn total_words(history: &History) -> i64 {
        history.database.with(|db| stats::total_words(db)).unwrap()
    }

    #[test]
    fn stores_the_transcript_and_its_recording() {
        let f = fixture();
        let id = f
            .history
            .record(dictation("Ship it today.", 2), HistoryStatus::Inserted)
            .id
            .unwrap();
        let list = f.history.list("").unwrap();
        assert_eq!(list.entries.len(), 1);
        assert!(list.entries[0].has_audio);
        assert_eq!(list.entries[0].retry, None);
        assert!(f.history.audio(id).unwrap().starts_with(b"RIFF"));
        let extracted = f.history.extract(id, f.dir.path()).unwrap();
        assert!(extracted.to_string_lossy().ends_with(".wav"));
        assert_eq!(total_words(&f.history), 3);
        assert!(f.changes.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    fn failed_entries_with_five_seconds_can_be_retried() {
        let f = fixture();
        let short = f
            .history
            .record(dictation("", 3), HistoryStatus::Failed)
            .id
            .unwrap();
        let long = f
            .history
            .record(dictation("", 6), HistoryStatus::Empty)
            .id
            .unwrap();
        assert!(matches!(
            f.history.retry_clip(short),
            Err(HistoryError::NotRetryable)
        ));
        let clip = f.history.retry_clip(long).unwrap();
        assert_eq!(clip.samples().len(), 16_000 * 6);
        assert_eq!(total_words(&f.history), 0, "failures don't count");
    }

    #[test]
    fn usage_categories_count_even_when_nothing_is_stored() {
        let f = fixture();
        f.settings
            .update(&serde_json::json!({"privacy": {"history": "never"}}))
            .unwrap();
        let mut email = dictation("dear team", 1);
        email.usage = Some(UsageCategory::Emails);
        f.history.record(email, HistoryStatus::Inserted);
        f.history
            .record(dictation("retried", 1), HistoryStatus::Inserted);
        let usage = f.history.database.with(|db| stats::usage(db)).unwrap();
        assert_eq!(
            usage.len(),
            1,
            "a dictation without a category isn't counted"
        );
        assert_eq!(
            (
                usage[0].category.as_str(),
                usage[0].words,
                usage[0].dictations
            ),
            ("emails", 2, 1)
        );
    }

    #[test]
    fn interrupted_recordings_are_recovered() {
        let f = fixture();
        let mut journal = f.history.start_journal().unwrap();
        let format = crate::audio::AudioFormat {
            sample_rate: 16_000,
            channels: 1,
        };
        journal.write(format, &[0.2; 16_000]);
        drop(journal);
        assert_eq!(f.history.recover_interrupted(), 1);
        let list = f.history.list("").unwrap();
        assert_eq!(list.entries[0].entry.status, HistoryStatus::Interrupted);
        assert_eq!(list.entries[0].retry, Some(RetryKind::Recover));
        assert_eq!(f.history.recover_interrupted(), 0);
    }

    #[test]
    fn cancelled_recordings_can_be_recovered_and_count_once_transcribed() {
        let f = fixture();
        let clip = AudioClip::from_mono_16k(vec![0.1; 8_000]);
        let id = f.history.record_cancelled(&clip, None).unwrap();
        let item = f.history.get(id).unwrap();
        assert_eq!(item.entry.status, HistoryStatus::Cancelled);
        assert_eq!(item.entry.audio_ms, 500);
        assert_eq!(item.retry, Some(RetryKind::Recover), "any length");
        assert_eq!(f.history.retry_clip(id).unwrap().samples().len(), 8_000);
        assert_eq!(total_words(&f.history), 0);
        let mut pasted = dictation("Pasted at last.", 1);
        pasted.replaces = Some(id);
        f.history.record(pasted, HistoryStatus::Inserted);
        assert_eq!(total_words(&f.history), 3);
        assert_eq!(f.history.list("").unwrap().entries.len(), 1);

        f.settings
            .update(&serde_json::json!({"privacy": {"history": "never"}}))
            .unwrap();
        assert!(!f.history.stores_dictations());
        assert_eq!(f.history.record_cancelled(&clip, None), None);
    }

    #[test]
    fn a_retranscription_updates_its_entry() {
        let f = fixture();
        let id = f
            .history
            .record(dictation("", 6), HistoryStatus::Failed)
            .id
            .unwrap();
        let mut again = dictation("Second try.", 6);
        again.replaces = Some(id);
        assert_eq!(
            f.history.record(again, HistoryStatus::Transcribed).id,
            Some(id)
        );
        let item = f.history.get(id).unwrap();
        assert_eq!(item.entry.final_text, "Second try.");
        assert_eq!(item.retry, None);
        assert_eq!(f.history.list("").unwrap().entries.len(), 1);
    }
}
