//! Delivers dictation state, levels and notices to the webviews as Tauri
//! events (names are documented in docs/ARCHITECTURE.md).

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter};

use super::flow_bar::FLOW_BAR_LABEL;
use crate::dictation::{Answer, DictationState, DictationUi, RecordingStatus, TransformView};
use crate::notify::Notice;

pub const STATE_EVENT: &str = "dictation://state";
pub const LEVEL_EVENT: &str = "dictation://level";
pub const NOTICE_EVENT: &str = "notify://notice";
pub const MODELS_EVENT: &str = "models://status";
pub const NAVIGATE_EVENT: &str = "app://navigate";
pub const SETTINGS_EVENT: &str = "settings://changed";
pub const ANSWER_EVENT: &str = "command://answer";
pub const CHANGES_EVENT: &str = "transform://changes";
pub const RECORDING_EVENT: &str = "shortcuts://recording";
pub const DISMISS_EVENT: &str = "notify://dismiss";
pub const MIC_LEVEL_EVENT: &str = "mic://level";
pub const MICROPHONES_EVENT: &str = "mic://devices";
pub const FLOW_BAR_HOVER_EVENT: &str = "flow-bar://hover";

pub struct TauriUi {
    app: AppHandle,
    /// Notices raised before the Flow Bar has loaded wait here.
    flow_bar_ready: AtomicBool,
    pending: Mutex<Vec<Notice>>,
}

impl TauriUi {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            flow_bar_ready: AtomicBool::new(false),
            pending: Mutex::new(Vec::new()),
        }
    }

    /// The Flow Bar is listening: deliver anything that arrived earlier.
    pub fn flow_bar_ready(&self) {
        self.flow_bar_ready.store(true, Ordering::SeqCst);
        let pending = std::mem::take(&mut *self.lock_pending());
        for notice in pending {
            self.emit_notice(&notice);
        }
    }

    fn emit_notice(&self, notice: &Notice) {
        if let Err(error) = self.app.emit(NOTICE_EVENT, notice) {
            log::warn!("could not emit a notice: {error}");
        }
    }

    fn lock_pending(&self) -> std::sync::MutexGuard<'_, Vec<Notice>> {
        self.pending.lock().unwrap_or_else(|err| err.into_inner())
    }
}

impl DictationUi for TauriUi {
    fn state(&self, state: DictationState) {
        if let Err(error) = self.app.emit(STATE_EVENT, state) {
            log::warn!("could not emit the dictation state: {error}");
        }
    }

    fn level(&self, level: f32) {
        let _ = self.app.emit_to(FLOW_BAR_LABEL, LEVEL_EVENT, level);
    }

    fn answer(&self, answer: Answer) {
        if let Err(error) = self.app.emit_to(FLOW_BAR_LABEL, ANSWER_EVENT, answer) {
            log::warn!("could not show the answer: {error}");
        }
    }

    fn changes(&self, view: TransformView) {
        if let Err(error) = self.app.emit_to(FLOW_BAR_LABEL, CHANGES_EVENT, view) {
            log::warn!("could not show the changes: {error}");
        }
    }

    fn shortcut_recording(&self, status: RecordingStatus) {
        if let Err(error) = self.app.emit(RECORDING_EVENT, status) {
            log::warn!("could not report the shortcut recording: {error}");
        }
    }

    fn dismiss(&self, id: u64) {
        let _ = self.app.emit(DISMISS_EVENT, id);
    }

    fn notice(&self, notice: Notice) {
        log::info!("notice ({:?}): {}", notice.level, notice.message);
        if self.flow_bar_ready.load(Ordering::SeqCst) {
            self.emit_notice(&notice);
        } else {
            self.lock_pending().push(notice);
        }
    }
}
