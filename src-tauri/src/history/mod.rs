//! Transcript history (HIS-01..08): every dictation with its recording for
//! 14 days, retries, crash recovery, the storage choice in Data and privacy,
//! and a daily database backup. Usage stats are counted here too, apart
//! from the transcripts, so they survive deletion (STA-05).

pub mod fixes;
pub mod journal;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{Datelike, Local, TimeZone, Timelike};
use serde::Serialize;

use crate::audio::{AudioClip, read_wav, wav_encode};
use crate::insights::{self, Milestone};
use crate::llm::LlmProviderId;
use crate::platform::AppIdentity;
use crate::settings::{HistoryStorage, SettingsStore};
use crate::storage::{
    Database, HistoryEntry, HistoryStatus, NewEntry, Retranscribed, StorageError, backup, history,
    now_ms, stats,
};
use crate::stt::SttEngineId;
use journal::Journal;

/// Recordings are kept this long for Retry and Play audio (HIS-04).
pub const AUDIO_DAYS: i64 = 14;
/// Failed or empty entries need this much audio to be retried (HIS-05).
pub const MIN_RETRY_MS: i64 = 5_000;
const DAY_MS: i64 = 86_400_000;
const LIST_LIMIT: usize = 1_000;

/// What a row offers for its audio. Mirrors `RetryKind` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RetryKind {
    /// A failed or empty transcription (HIS-05).
    Retry,
    /// A recording interrupted by a quit or crash (HIS-06).
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
    #[error("Cannot retry transcription as audio is no longer available (older than 14 days)")]
    RetryAudioGone,
    #[error("Can't play audio because it's no longer available (older than 14 days)")]
    PlayAudioGone,
    #[error("Cannot download audio as it is no longer available (older than 14 days)")]
    DownloadAudioGone,
    #[error("This transcript can't be retried.")]
    NotRetryable,
    #[error("That transcript no longer exists.")]
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

    /// Where the recording in progress is saved as it arrives (HIS-06);
    /// nothing is saved with "Never store".
    pub fn start_journal(&self) -> Option<Journal> {
        (self.storage() != HistoryStorage::Never).then(|| Journal::new(&self.recordings, now_ms()))
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

    /// Applies Data and privacy › Local data storage (HIS-07): "Never store"
    /// deletes every transcript, recording and backup; "Auto-delete" deletes
    /// what is older than 24 hours now.
    pub fn apply_storage(&self) {
        match self.storage() {
            HistoryStorage::Never => self.delete_everything(),
            HistoryStorage::AutoDelete => self.sweep(),
            HistoryStorage::Local => {}
        }
    }

    fn delete_everything(&self) {
        match self.database.with(|db| history::delete_before(db, None)) {
            Ok(audio) if !audio.is_empty() => self.delete_recordings(audio),
            Ok(_) => {}
            Err(error) => log::warn!("could not delete the history: {error}"),
        }
        self.delete_orphans();
        journal::discard_all(&self.recordings);
        backup::delete_all(&self.backups);
        (self.changed)();
    }

    /// The hourly chores: recordings older than 14 days, entries older than
    /// 24 hours with auto-delete, stray files, and the daily backup.
    pub fn sweep(&self) {
        let now = now_ms();
        match self
            .database
            .with(|db| history::expire_audio(db, now - AUDIO_DAYS * DAY_MS))
        {
            Ok(audio) => self.delete_recordings(audio),
            Err(error) => log::warn!("could not expire old recordings: {error}"),
        }
        if self.storage() == HistoryStorage::AutoDelete {
            match self
                .database
                .with(|db| history::delete_before(db, Some(now - DAY_MS)))
            {
                Ok(audio) => {
                    let deleted = !audio.is_empty();
                    self.delete_recordings(audio);
                    if deleted {
                        (self.changed)();
                    }
                }
                Err(error) => log::warn!("could not auto-delete the history: {error}"),
            }
        }
        self.delete_orphans();
        self.backup();
    }

    /// Today's backup, if it hasn't been made (HIS-08).
    pub fn backup(&self) {
        let today = Local::now().format("%Y-%m-%d").to_string();
        match self
            .database
            .with(|db| backup::daily(db, &self.backups, &today))
        {
            Ok(Some(path)) => log::info!("backed up the database to {}", path.display()),
            Ok(None) => {}
            Err(error) => log::warn!("could not back up the database: {error}"),
        }
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

    fn delete_recordings(&self, names: Vec<String>) {
        for name in names {
            let path = self.recordings.join(&name);
            if let Err(error) = std::fs::remove_file(&path)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                log::warn!("could not delete {}: {error}", path.display());
            }
        }
    }

    /// Recordings no entry names any more (a crash between saving and linking).
    fn delete_orphans(&self) {
        let Ok(known) = self.database.with(|db| history::audio_paths(db)) else {
            return;
        };
        let Ok(files) = std::fs::read_dir(&self.recordings) else {
            return;
        };
        let orphans = files
            .filter_map(Result::ok)
            .filter_map(|file| file.file_name().to_str().map(str::to_owned))
            .filter(|name| is_recording_name(name) && !known.contains(name))
            .collect();
        self.delete_recordings(orphans);
    }
}

/// `<id>.wav`; journals and anything else are left alone.
fn is_recording_name(name: &str) -> bool {
    name.strip_suffix(".wav")
        .is_some_and(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
}

fn item(entry: HistoryEntry, now: i64) -> HistoryItem {
    let has_audio = entry.audio_path.is_some() && now - entry.created_at < AUDIO_DAYS * DAY_MS;
    let retry = match entry.status {
        _ if !has_audio => None,
        HistoryStatus::Interrupted => Some(RetryKind::Recover),
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

    struct Fixture {
        history: Arc<History>,
        settings: Arc<SettingsStore>,
        changes: Arc<AtomicUsize>,
        dir: tempfile::TempDir,
    }

    fn fixture() -> Fixture {
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

    fn dictation(text: &str, seconds: usize) -> Dictation {
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

    fn total_words(history: &History) -> i64 {
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
    fn never_store_deletes_everything_but_keeps_counting() {
        let f = fixture();
        let id = f
            .history
            .record(dictation("one two", 1), HistoryStatus::Inserted)
            .id
            .unwrap();
        f.history.backup();
        f.settings
            .update(&serde_json::json!({"privacy": {"history": "never"}}))
            .unwrap();
        f.history.apply_storage();
        assert!(f.history.list("").unwrap().disabled);
        assert!(matches!(f.history.audio(id), Err(HistoryError::NotFound)));
        assert!(backup::copies(&f.dir.path().join("backups")).is_empty());
        assert_eq!(
            f.history
                .record(dictation("three", 1), HistoryStatus::Inserted)
                .id,
            None
        );
        assert_eq!(total_words(&f.history), 3, "stats survive (STA-05)");
        let recordings = std::fs::read_dir(f.dir.path().join("recordings")).unwrap();
        assert_eq!(recordings.count(), 0);
    }

    #[test]
    fn auto_delete_removes_entries_older_than_a_day() {
        let f = fixture();
        let mut old = dictation("old", 1);
        old.created_at = now_ms() - 2 * DAY_MS;
        f.history.record(old, HistoryStatus::Inserted);
        f.history
            .record(dictation("new", 1), HistoryStatus::Inserted);
        f.settings
            .update(&serde_json::json!({"privacy": {"history": "autoDelete"}}))
            .unwrap();
        f.history.apply_storage();
        let list = f.history.list("").unwrap();
        assert_eq!(list.entries.len(), 1);
        assert_eq!(list.entries[0].entry.final_text, "new");
    }

    #[test]
    fn recordings_expire_after_fourteen_days() {
        let f = fixture();
        let mut old = dictation("", 6);
        old.created_at = now_ms() - 15 * DAY_MS;
        let id = f.history.record(old, HistoryStatus::Failed).id.unwrap();
        f.history.sweep();
        let item = f.history.get(id).unwrap();
        assert!(!item.has_audio);
        assert!(matches!(
            f.history.retry_clip(id),
            Err(HistoryError::RetryAudioGone)
        ));
        assert!(matches!(
            f.history.audio(id),
            Err(HistoryError::PlayAudioGone)
        ));
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
