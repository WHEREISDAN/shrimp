use std::time::Duration;

use serde::Serialize;

use super::gestures::{BarControl, RecordingMode};
use crate::audio::AudioClip;
use crate::hotkeys::{Action, ShortcutEvent};
use crate::notify::Notice;
use crate::platform::{CopyReason, InsertText};

/// What the Flow Bar shows. Mirrors `DictationState` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "phase")]
pub enum DictationState {
    Idle,
    Recording { mode: Mode },
    Processing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    Hold,
    Locked,
}

impl From<RecordingMode> for Mode {
    fn from(mode: RecordingMode) -> Self {
        match mode {
            RecordingMode::Hold => Self::Hold,
            RecordingMode::Locked => Self::Locked,
        }
    }
}

/// Where the controller reports to; the app emits Tauri events, tests record.
pub trait DictationUi: Send + Sync {
    fn state(&self, state: DictationState);
    /// Input level in 0..=1, about 30 times a second while recording.
    fn level(&self, level: f32);
    fn notice(&self, notice: Notice);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// Warning sound and notice before the limit (CORE-08).
    pub session_warning: Duration,
    pub session_limit: Duration,
    /// How long the transcript stays on the clipboard before the old
    /// contents return (INS-02).
    pub clipboard_hold: Duration,
    /// The same for remote desktops, which read the clipboard late (INS-10).
    pub remote_clipboard_hold: Duration,
    pub level_interval: Duration,
    /// How often to retry the keyboard hook while Accessibility is missing.
    pub hotkey_retry: Duration,
    /// Lets focus return to the previous app after a tray menu closes.
    pub menu_paste_delay: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            session_warning: Duration::from_secs(19 * 60),
            session_limit: Duration::from_secs(20 * 60),
            clipboard_hold: Duration::from_millis(500),
            remote_clipboard_hold: Duration::from_secs(5),
            level_interval: Duration::from_millis(33),
            hotkey_retry: Duration::from_secs(2),
            menu_paste_delay: Duration::from_millis(250),
        }
    }
}

/// Everything the controller task reacts to, on one channel.
#[derive(Debug)]
pub enum ControllerInput {
    Shortcut(ShortcutEvent),
    Bar(BarControl),
    /// Paste or copy last transcript from the tray menu.
    Menu(Action),
    TapTimeout(u64),
    SessionWarning(u64),
    SessionLimit(u64),
    RetryHotkeys,
    /// A cancelled recording, kept so paste-last can transcribe it.
    Cancelled(AudioClip),
    Processed(Processed),
}

/// A finished pipeline run and the notices it raised on the way (an AI
/// edit that fell back, a cloud engine that failed over to Whisper).
#[derive(Debug)]
pub struct Processed {
    pub outcome: Outcome,
    pub notices: Vec<Notice>,
}

impl From<Outcome> for Processed {
    fn from(outcome: Outcome) -> Self {
        Self {
            outcome,
            notices: Vec::new(),
        }
    }
}

/// The final text of a dictation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivered {
    pub text: InsertText,
    /// False when AI edits were off, skipped or failed (FMT-10).
    pub formatted: bool,
}

/// How a transcription and insertion ended.
#[derive(Debug)]
pub enum Outcome {
    Inserted(Delivered),
    /// The audio source failed while recording (e.g. the mic disconnected).
    RecordingFailed(String),
    Copied(Delivered, CopyReason),
    NoSpeech,
    /// The edits left nothing to insert ("never mind").
    NothingToInsert,
    SttFailed {
        clip: AudioClip,
        message: String,
        unavailable: bool,
    },
    InsertFailed(Delivered, String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastStatus {
    Inserted,
    Copied,
    Failed,
    Cancelled,
}

/// The most recent dictation, for paste and copy last transcript (INS-05/06).
#[derive(Debug, Clone)]
pub struct LastDictation {
    pub status: LastStatus,
    pub text: Option<Delivered>,
    /// Kept for failed and cancelled dictations, so paste-last can retry.
    pub clip: Option<AudioClip>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_timing_matches_the_documented_limits() {
        let timing = Timing::default();
        assert_eq!(timing.session_warning, Duration::from_secs(19 * 60));
        assert_eq!(timing.session_limit, Duration::from_secs(20 * 60));
        assert_eq!(timing.clipboard_hold, Duration::from_millis(500));
    }

    #[test]
    fn state_serializes_with_a_phase_tag() {
        let json = serde_json::to_value(DictationState::Recording { mode: Mode::Locked }).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"phase": "recording", "mode": "locked"})
        );
        let json = serde_json::to_value(DictationState::Processing).unwrap();
        assert_eq!(json, serde_json::json!({"phase": "processing"}));
    }
}
