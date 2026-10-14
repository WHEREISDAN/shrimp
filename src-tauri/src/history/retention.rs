//! How long History keeps things: Data and privacy's Local data storage
//! (HIS-07) and Delete all transcripts (HIS-09), the hourly sweep of old
//! recordings and entries (HIS-04), and the daily backup (HIS-08).

use chrono::Local;

use super::{AUDIO_DAYS, DAY_MS, History, HistoryError, journal};
use crate::settings::HistoryStorage;
use crate::storage::{backup, history, now_ms};

impl History {
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
        if let Err(error) = self.delete_transcripts() {
            log::warn!("could not delete the history: {error}");
        }
        journal::discard_all(&self.recordings);
        backup::delete_all(&self.backups);
        (self.changed)();
    }

    /// Data and privacy › Delete all transcripts (HIS-09): every entry and
    /// its recording. Insights, the dictionary, snippets and settings stay;
    /// the backups held the transcripts too, so today's is made again
    /// without them. A recording in progress keeps its journal.
    pub fn delete_all(&self) -> Result<(), HistoryError> {
        self.delete_transcripts()?;
        backup::delete_all(&self.backups);
        self.backup();
        (self.changed)();
        Ok(())
    }

    fn delete_transcripts(&self) -> Result<(), HistoryError> {
        let audio = self.database.with(|db| history::delete_before(db, None))?;
        self.delete_recordings(audio);
        self.delete_orphans();
        Ok(())
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

    pub(super) fn delete_recordings(&self, names: Vec<String>) {
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

#[cfg(test)]
mod tests {
    use super::super::tests::{dictation, fixture, total_words};
    use super::*;
    use crate::storage::HistoryStatus;

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
    fn delete_all_keeps_storing_and_counting() {
        let f = fixture();
        let id = f
            .history
            .record(dictation("one two", 1), HistoryStatus::Inserted)
            .id
            .unwrap();
        f.history.backup();
        let backups = f.dir.path().join("backups");
        let before = std::fs::read(&backup::copies(&backups)[0]).unwrap();
        let mut journal = f.history.start_journal().unwrap();
        journal.write(
            crate::audio::AudioFormat {
                sample_rate: 16_000,
                channels: 1,
            },
            &[0.2; 1_600],
        );
        let changes = f.changes.load(std::sync::atomic::Ordering::SeqCst);

        f.history.delete_all().unwrap();
        let list = f.history.list("").unwrap();
        assert!(list.entries.is_empty());
        assert!(!list.disabled, "Local data storage is unchanged");
        assert!(matches!(f.history.audio(id), Err(HistoryError::NotFound)));
        assert_eq!(total_words(&f.history), 2, "Insights keep their counts");
        assert!(f.changes.load(std::sync::atomic::Ordering::SeqCst) > changes);
        let copies = backup::copies(&backups);
        assert_eq!(copies.len(), 1, "today's backup is made again");
        assert_ne!(std::fs::read(&copies[0]).unwrap(), before);
        let recordings: Vec<String> = std::fs::read_dir(f.dir.path().join("recordings"))
            .unwrap()
            .map(|file| file.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            recordings.iter().all(|name| name.starts_with("active-")),
            "only the recording in progress is left: {recordings:?}"
        );
        drop(journal);
        assert!(
            f.history
                .record(dictation("three", 1), HistoryStatus::Inserted)
                .id
                .is_some()
        );
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
}
