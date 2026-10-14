//! Settings › Notifications (NOT-01): notices in a switched-off category are
//! dropped before they sound or show, and so is everything but status
//! notices during onboarding. Errors, permissions and everything a
//! dictation reports are status notices and always show.

use std::sync::Arc;

use super::state::{Answer, DictationState, DictationUi, RecordingStatus};
use super::transform::TransformView;
use crate::notify::{Notice, NoticeCategory};
use crate::settings::SettingsStore;

pub struct MutedNotices {
    inner: Arc<dyn DictationUi>,
    settings: Arc<SettingsStore>,
}

impl MutedNotices {
    pub fn new(inner: Arc<dyn DictationUi>, settings: Arc<SettingsStore>) -> Self {
        Self { inner, settings }
    }
}

impl DictationUi for MutedNotices {
    fn state(&self, state: DictationState) {
        self.inner.state(state);
    }

    fn level(&self, level: f32) {
        self.inner.level(level);
    }

    fn notice(&self, notice: Notice) {
        let settings = self.settings.get();
        // During onboarding only status notices show: permissions, the microphone, errors.
        let allowed = notice.category.allowed(&settings.notifications)
            && (settings.onboarding.completed || notice.category == NoticeCategory::Status);
        if allowed {
            self.inner.notice(notice);
        } else {
            log::info!("muted ({:?}): {}", notice.category, notice.message);
        }
    }

    fn dismiss(&self, id: u64) {
        self.inner.dismiss(id);
    }

    fn answer(&self, answer: Answer) {
        self.inner.answer(answer);
    }

    fn changes(&self, view: TransformView) {
        self.inner.changes(view);
    }

    fn shortcut_recording(&self, status: RecordingStatus) {
        self.inner.shortcut_recording(status);
    }

    fn last_transcript(&self, paste: bool, copy: bool) {
        self.inner.last_transcript(paste, copy);
    }
}
