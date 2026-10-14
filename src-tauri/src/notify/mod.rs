//! Notices shown next to the Flow Bar (BAR-03). They never take focus and
//! never appear as modal dialogs.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// Mirrors `Notice` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub id: u64,
    pub level: NoticeLevel,
    pub message: String,
    /// Buttons, in order.
    pub actions: Vec<NoticeAction>,
    /// Which Settings › Notifications switch mutes it, if any (NOT-01).
    pub category: NoticeCategory,
    /// Comes with the notice sound (warnings and errors with no cue of their own).
    #[serde(skip)]
    pub sound: bool,
}

/// Settings › Notifications (NOT-01). Status notices (errors, permissions,
/// the microphone, what a dictation did) can't be muted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NoticeCategory {
    #[default]
    Status,
    /// Setup and usage tips.
    Suggestion,
    /// New features and formatting tips.
    Announcement,
    /// Word counts and the voice profile.
    Milestone,
}

impl NoticeCategory {
    /// Whether the user's switches let this category through.
    pub fn allowed(self, settings: &crate::settings::NotificationSettings) -> bool {
        match self {
            Self::Status => true,
            Self::Suggestion => settings.suggestions,
            Self::Announcement => settings.announcements,
            Self::Milestone => settings.milestones,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NoticeLevel {
    Info,
    Warning,
    Error,
}

/// A button on a notice; the Flow Bar sends it back through `run_notice_action`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoticeAction {
    CopyLastTranscript,
    OpenAccessibilitySettings,
    OpenMicrophoneSettings,
    OpenKeyboardSettings,
    RetryModelDownload,
    /// Opens Shrimp's settings at the AI provider section.
    OpenAiSettings,
    /// Opens Settings › Experimental (Command Mode is off).
    OpenExperimentalSettings,
    /// Shows what the last transform changed (TRF-04).
    ViewTransformChanges,
    /// The same, after auto-apply changed a dictation (TRF-05).
    RevealEdit,
    /// Opens the dictionary (words were learned, DIC-07).
    OpenDictionary,
    /// Transcribes the cancelled dictation and inserts it (CORE-11).
    UndoCancel,
    /// Opens Settings › Microphone (MIC-07).
    ChooseMicrophone,
    /// The same, from the Bluetooth advice (MIC-08).
    SwitchMicrophone,
    /// Uses the microphone just connected (MIC-04).
    SwitchToNewMicrophone,
    /// Keeps the current microphone; the new one is ranked last.
    KeepMicrophone,
    /// Stops the Bluetooth advice.
    DismissBluetoothAdvice,
    /// Opens System Settings › Sound › Input.
    OpenSoundSettings,
    /// The same, from a warning about no sound.
    TroubleshootMicrophone,
    /// Opens Settings › Languages (LANG-04).
    OpenLanguageSettings,
    /// Adds the language the dictation sounded like.
    AddDetectedLanguage,
    /// Stops the different-language card.
    DismissLanguageCard,
    /// Shows the Flow Bar again after Hide for 1 hour (BAR-04).
    ShowFlowBar,
    /// Opens Home's history (HIS-06).
    OpenHistory,
    /// Opens Insights (a word-count milestone).
    OpenInsights,
    /// Opens the voice profile, which can now be generated (STA-04).
    SeeVoiceProfile,
    /// Opens the voice profile, which can now be updated.
    UpdateVoiceProfile,
    /// Installs the downloaded update and restarts (PKG-03).
    RestartToUpdate,
    /// Great: turns the "press enter" command on and presses Enter (CORE-17).
    EnablePressEnter,
    /// Disable: types " press enter" back and leaves the command off.
    DisablePressEnter,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl Notice {
    pub fn new(level: NoticeLevel, message: impl Into<String>) -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            level,
            message: message.into(),
            actions: Vec::new(),
            category: NoticeCategory::Status,
            sound: level != NoticeLevel::Info,
        }
    }

    pub fn in_category(mut self, category: NoticeCategory) -> Self {
        self.category = category;
        self
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(NoticeLevel::Info, message)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(NoticeLevel::Warning, message)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(NoticeLevel::Error, message)
    }

    pub fn with_action(mut self, action: NoticeAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Shown without the notice sound, because a cue already played.
    pub fn without_sound(mut self) -> Self {
        self.sound = false;
        self
    }

    /// The first button, if any.
    pub fn action(&self) -> Option<NoticeAction> {
        self.actions.first().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_for_the_flow_bar() {
        let notice = Notice::warning("No text field").with_action(NoticeAction::CopyLastTranscript);
        let json = serde_json::to_value(&notice).unwrap();
        assert_eq!(json["level"], "warning");
        assert_eq!(json["message"], "No text field");
        assert_eq!(json["actions"], serde_json::json!(["copyLastTranscript"]));
        assert_eq!(json["category"], "status");
        assert!(json["id"].as_u64().unwrap() > 0);
        let reveal: NoticeAction = serde_json::from_str("\"revealEdit\"").unwrap();
        assert_eq!(reveal, NoticeAction::RevealEdit);
        assert_eq!(serde_json::to_value(reveal).unwrap(), "revealEdit");
    }

    #[test]
    fn only_tips_announcements_and_milestones_can_be_muted() {
        let muted = crate::settings::NotificationSettings {
            suggestions: false,
            announcements: false,
            milestones: false,
        };
        assert!(NoticeCategory::Status.allowed(&muted));
        assert!(!NoticeCategory::Suggestion.allowed(&muted));
        assert!(!NoticeCategory::Milestone.allowed(&muted));
        assert!(NoticeCategory::Announcement.allowed(&Default::default()));
    }

    #[test]
    fn ids_are_unique() {
        assert_ne!(Notice::info("a").id, Notice::info("b").id);
    }
}
