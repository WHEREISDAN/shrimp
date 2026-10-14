use std::time::Duration;

use serde::Serialize;

use super::gestures::{BarControl, RecordingMode};
use super::learn::{LearnTarget, LearnTiming};
use super::transform::{TransformRun, TransformView};
use crate::audio::AudioClip;
use crate::edit::transforms::QuickTransform;
use crate::history::Dictation;
use crate::history::journal::JournalFiles;
use crate::hotkeys::{Action, RecordUpdate, ShortcutEvent};
use crate::notify::Notice;
use crate::platform::{CopyReason, InsertText};

/// What the Flow Bar shows. Mirrors `DictationState` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "phase")]
pub enum DictationState {
    Idle,
    /// `command` marks a Command Mode session (CMD-01).
    Recording {
        mode: Mode,
        command: bool,
    },
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
    /// Takes a notice away once its cause is gone.
    fn dismiss(&self, _id: u64) {}
    /// A Command Mode answer, shown in a panel instead of pasted (CMD-03).
    fn answer(&self, _answer: Answer) {}
    /// What the last transform changed (TRF-04).
    fn changes(&self, _view: TransformView) {}
    /// Progress of a shortcut being recorded (KEYS-01).
    fn shortcut_recording(&self, _status: RecordingStatus) {}
    /// Paste last transcript has something to paste (WIN-09), and Copy
    /// last transcript some text to copy (APP-10).
    fn last_transcript(&self, _paste: bool, _copy: bool) {}
}

/// A shortcut being recorded, for the Shortcuts dialog. Mirrors
/// `RecordingStatus` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum RecordingStatus {
    /// Waiting for keys; `preview` is what is held so far.
    Recording {
        action: Action,
        preview: Option<String>,
    },
    Saved {
        action: Action,
        shortcut: String,
        /// Saved, but it may conflict (KEYS-07).
        warning: Option<String>,
    },
    /// Rejected; the previous binding is unchanged (KEYS-03).
    Failed {
        action: Action,
        message: String,
    },
    Cancelled {
        action: Action,
    },
}

/// A Command Mode answer for the Flow Bar's panel. Mirrors `Answer` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub id: u64,
    /// The spoken instruction.
    pub question: String,
    pub text: String,
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
    /// How the auto-learn watcher looks at the field (DIC-07).
    pub learn: LearnTiming,
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
            learn: LearnTiming::default(),
        }
    }
}

/// Everything the controller task reacts to, on one channel.
#[derive(Debug)]
pub enum ControllerInput {
    Shortcut(ShortcutEvent),
    /// A key that is no shortcut went down during a held shortcut (CORE-12).
    OtherKey,
    /// Record a shortcut for `action`, replacing the one at `index` or adding one.
    RecordShortcut {
        action: Action,
        index: Option<usize>,
    },
    /// The dialog's ✕, a click or closing it.
    CancelRecording,
    Recording(RecordUpdate),
    /// Another app turned Secure Keyboard Entry on (its name) or off.
    SecureInput(Option<String>),
    /// The cancel notice's Undo: transcribe and insert the cancelled audio (CORE-11).
    UndoCancel,
    /// What Whisper heard after a dictation (LANG-04).
    LanguageDetected {
        code: String,
        probability: f32,
    },
    /// The different-language card's Add language.
    AddDetectedLanguage,
    /// The "press enter" prompt's Great (true) or Disable (CORE-17).
    AnswerPressEnter {
        enable: bool,
    },
    /// "Switch to Canadian English?"'s Switch (LANG-07).
    SwitchToCanadianEnglish,
    /// The most buttons a connected mouse has, read after a dictation (CORE-18).
    MouseButtons(u32),
    Bar(BarControl),
    /// Paste or copy last transcript from the tray menu.
    Menu(Action),
    TapTimeout(u64),
    SessionWarning(u64),
    SessionLimit(u64),
    RetryHotkeys,
    /// Shortcuts in the settings may have changed.
    SettingsChanged,
    /// The changes view's buttons and "See changes" (TRF-04).
    Transform(TransformAction),
    /// The Flow Bar's wand runs its chosen transform (TRF-07).
    Wand,
    /// A cancelled recording, kept so paste-last can transcribe it, and
    /// the History entry it was stored as (HIS-10).
    Cancelled {
        clip: AudioClip,
        history_id: Option<i64>,
    },
    /// History's Retry or Recover for an entry (HIS-05, HIS-06).
    RetryHistory(i64),
    Processed(Box<Processed>),
    /// Ends the controller task; the sender hears once every service it held
    /// is released.
    Shutdown(std::sync::mpsc::Sender<()>),
}

/// What the user can do with the last transform. Mirrors `TransformAction`
/// in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransformAction {
    View,
    Copy,
    Retry,
    Undo,
    /// Runs another transform on the original text in place (TRF-04).
    Retransform(QuickTransform),
}

/// A finished pipeline run and the notices it raised on the way (an AI
/// edit that fell back, a cloud engine that failed over to Whisper).
#[derive(Debug)]
pub struct Processed {
    pub outcome: Outcome,
    pub notices: Vec<Notice>,
    /// Where a dictation was pasted, to learn from corrections (DIC-07).
    pub learn: Option<LearnTarget>,
    /// The audio, to check afterwards whether another language was spoken (LANG-04).
    pub language_check: Option<AudioClip>,
    /// What History stores (HIS-01).
    pub record: Option<Dictation>,
    /// The History entry it was stored as.
    pub history_id: Option<i64>,
    /// The recording saved as it happened, deleted once stored (HIS-06).
    pub journal: Option<JournalFiles>,
    /// The transform auto-apply ran on the dictation, for the changes view (TRF-05).
    pub transform: Option<Box<TransformRun>>,
    /// How the dictation started; None for everything but dictations.
    pub started_by: Option<StartedBy>,
}

impl From<Outcome> for Processed {
    fn from(outcome: Outcome) -> Self {
        Self {
            outcome,
            notices: Vec::new(),
            learn: None,
            language_check: None,
            record: None,
            history_id: None,
            journal: None,
            transform: None,
            started_by: None,
        }
    }
}

/// What started a recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartedBy {
    /// A key or mouse shortcut.
    Shortcut,
    /// A click on the Flow Bar.
    FlowBar,
}

/// The final text of a dictation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
    /// "Press enter" alone: Enter was pressed, nothing pasted (CORE-16).
    EnterPressed,
    /// "Press enter" alone with the command off: nothing pasted, and a
    /// notice asks whether to turn the command on (CORE-17).
    PressEnterAsked,
    /// A Command Mode answer to show (CMD-03).
    Answered(Answer),
    /// A Command Mode search opened in the browser (CMD-05).
    Searched,
    /// Command Mode could not run; nothing was changed.
    CommandFailed(String),
    /// A transform replaced the text (TRF-01..03).
    Transformed(Box<TransformRun>),
    /// A transform could not run; the message says why (TRF-06).
    TransformFailed(String),
    /// The transform's result was swapped back for the original.
    TransformUndone,
    /// The text moved before the result could replace it; it is on the clipboard.
    TransformCopied(Delivered),
    /// A History entry was transcribed again; nothing was pasted (HIS-05).
    Retried,
    /// History's Retry could not run or failed; the message says why.
    RetryFailed(String),
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
    /// Its History entry, which a retry updates.
    pub history_id: Option<i64>,
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
    fn transform_actions_deserialize_from_the_changes_view() {
        let parse = |json| serde_json::from_value::<TransformAction>(json).unwrap();
        assert_eq!(parse(serde_json::json!("retry")), TransformAction::Retry);
        assert_eq!(
            parse(serde_json::json!({"retransform": "turnToList"})),
            TransformAction::Retransform(QuickTransform::TurnToList)
        );
    }

    #[test]
    fn state_serializes_with_a_phase_tag() {
        let json = serde_json::to_value(DictationState::Recording {
            mode: Mode::Locked,
            command: true,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({"phase": "recording", "mode": "locked", "command": true})
        );
        let json = serde_json::to_value(DictationState::Processing).unwrap();
        assert_eq!(json, serde_json::json!({"phase": "processing"}));
    }
}
