//! Cancelled dictations: Esc, the Flow Bar's ✕ or a third quick press, and
//! another app's shortcut early in a hold (CORE-12). The audio is kept for
//! the cancel notice's Undo and paste last (CORE-11) and, from half a
//! second on, stored in History to recover later (HIS-10).

use serde_json::json;

use super::{Controller, Recording};
use crate::audio::AudioClip;
use crate::dictation::gestures::CancelReason;
use crate::dictation::state::{ControllerInput, LastDictation, LastStatus};
use crate::history::MIN_CANCELLED_AUDIO;
use crate::history::journal::Journal;
use crate::hotkeys::Action;
use crate::notify::{Notice, NoticeAction};
use crate::settings::NoticeSettings;
use crate::sounds::Cue;
use crate::t;

/// "You can recover your cancelled transcript" shows at most this often,
/// and once a day (article 4363488655).
const RECOVER_CARDS: u32 = 3;

impl Controller {
    /// Ends a cancelled recording: the cue, the notice, and its audio set
    /// aside. Command Mode sessions aren't stored in History.
    pub(super) fn set_aside(&mut self, recording: Recording, reason: CancelReason, command: bool) {
        self.sounds.play(Cue::Cancel);
        let store = !command && self.stores_cancelled(&recording);
        let notice = match reason {
            CancelReason::Interrupted => Notice::info(t!("notice.dictation.interrupted")),
            _ if command => Notice::info(t!("notice.command.cancelled")),
            _ => self.cancel_notice(store),
        };
        self.ui.notice(notice);
        // Only a cancel on purpose replaces the last transcript (CORE-11).
        let remember = reason == CancelReason::User;
        if !store && !remember {
            recording.discard();
            return;
        }
        self.keep_audio(recording, store, remember);
    }

    fn stores_cancelled(&self, recording: &Recording) -> bool {
        recording.started.elapsed() >= MIN_CANCELLED_AUDIO
            && self
                .pipeline
                .history
                .as_ref()
                .is_some_and(|history| history.stores_dictations())
    }

    /// The cancel notice with Undo. The first few times a cancelled
    /// dictation is stored, it says so and offers Open History too.
    fn cancel_notice(&self, stored: bool) -> Notice {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let card = stored
            .then(|| recover_card(&self.settings.get().notices, &today))
            .flatten();
        let Some(patch) = card else {
            return Notice::info(t!("notice.dictation.cancelled"))
                .with_action(NoticeAction::UndoCancel);
        };
        if let Err(error) = self.settings.update(&patch) {
            log::warn!("could not count the notice: {error}");
        }
        Notice::info(t!("notice.dictation.recoverCancelled"))
            .with_action(NoticeAction::UndoCancel)
            .with_action(NoticeAction::OpenHistory)
    }

    /// Finishes the recording on the blocking pool, stores it when `store`,
    /// and hands it back for Undo and paste last when `remember`. The
    /// journal goes once the audio is safe (HIS-06).
    fn keep_audio(&self, recording: Recording, store: bool, remember: bool) {
        let Recording {
            recorder,
            journal,
            context,
            ..
        } = recording;
        let journal = journal.map(Journal::finish);
        let history = self.pipeline.history.clone().filter(|_| store);
        let app = context.app().cloned();
        let tx = self.tx.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let clip = recorder.finish().ok();
            let history_id = clip
                .as_ref()
                .zip(history)
                .and_then(|(clip, history)| history.record_cancelled(clip, app));
            if let Some(journal) = journal {
                journal.delete();
            }
            if let (Some(clip), true) = (clip, remember) {
                let _ = tx.send(ControllerInput::Cancelled { clip, history_id });
            }
        });
    }

    /// The cancelled audio becomes the last transcript; pasting it updates
    /// its History entry instead of adding one.
    pub(super) fn remember_cancelled(&mut self, clip: AudioClip, history_id: Option<i64>) {
        self.last = Some(LastDictation {
            status: LastStatus::Cancelled,
            text: None,
            clip: Some(clip),
            history_id,
        });
        self.ui.last_transcript(true);
    }

    /// Transcribes the cancelled dictation and inserts it (CORE-11).
    pub(super) fn undo_cancel(&mut self) {
        let cancelled = self
            .last
            .as_ref()
            .is_some_and(|last| last.status == LastStatus::Cancelled);
        if cancelled && !self.gestures.is_recording() && !self.gestures.is_busy() {
            self.handle(ControllerInput::Menu(Action::PasteLast));
        }
    }
}

/// The settings patch that counts the recover card, if it may show today.
fn recover_card(notices: &NoticeSettings, today: &str) -> Option<serde_json::Value> {
    if notices.cancel_recover_count >= RECOVER_CARDS || notices.cancel_recover_day == today {
        return None;
    }
    Some(json!({"notices": {
        "cancelRecoverCount": notices.cancel_recover_count + 1,
        "cancelRecoverDay": today,
    }}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_recover_card_shows_three_times_at_most_once_a_day() {
        let mut notices = NoticeSettings::default();
        let first = recover_card(&notices, "2026-10-13").unwrap();
        assert_eq!(first["notices"]["cancelRecoverCount"], 1);
        assert_eq!(first["notices"]["cancelRecoverDay"], "2026-10-13");
        notices.cancel_recover_count = 1;
        notices.cancel_recover_day = "2026-10-13".into();
        assert_eq!(recover_card(&notices, "2026-10-13"), None, "once a day");
        assert!(recover_card(&notices, "2026-10-14").is_some());
        notices.cancel_recover_count = 3;
        assert_eq!(recover_card(&notices, "2026-10-20"), None, "three in all");
    }
}
