//! The typed settings. Every section has defaults, so files written by older
//! versions (or missing fields) still load.

use serde::{Deserialize, Serialize};

use super::audio::{MediaSettings, MicrophoneSettings, SoundSettings};
use super::flow_bar::FlowBarSettings;
use super::languages::LanguageSettings;
use super::shortcuts::ShortcutSettings;
use super::transforms::TransformSettings;
use crate::edit::{AppCategory, CleanupLevel, Style};
use crate::llm::{LlmProviderId, catalog};
use crate::stt::{DEFAULT_WHISPER_MODEL, SttEngineId, find_whisper_model};
use crate::t;

pub const SETTINGS_VERSION: u32 = 1;
const MAX_MODEL_LENGTH: usize = 120;

/// Mirrors `Settings` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    /// Grows with every saved change, so the webview can drop a change
    /// event that arrives after a newer one.
    pub revision: u64,
    pub stt: SttSettings,
    pub ai: AiSettings,
    pub styles: StyleSettings,
    pub privacy: PrivacySettings,
    pub experimental: ExperimentalSettings,
    pub shortcuts: ShortcutSettings,
    pub notices: NoticeSettings,
    pub transforms: TransformSettings,
    pub vibe_coding: VibeCodingSettings,
    pub dictionary: DictionarySettings,
    pub microphone: MicrophoneSettings,
    pub sounds: SoundSettings,
    pub media: MediaSettings,
    pub languages: LanguageSettings,
    pub flow_bar: FlowBarSettings,
    pub notifications: NotificationSettings,
    pub app: AppSettings,
    pub onboarding: OnboardingSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            revision: 0,
            stt: SttSettings::default(),
            ai: AiSettings::default(),
            styles: StyleSettings::default(),
            privacy: PrivacySettings::default(),
            experimental: ExperimentalSettings::default(),
            shortcuts: ShortcutSettings::default(),
            notices: NoticeSettings::default(),
            transforms: TransformSettings::default(),
            vibe_coding: VibeCodingSettings::default(),
            dictionary: DictionarySettings::default(),
            microphone: MicrophoneSettings::default(),
            sounds: SoundSettings::default(),
            media: MediaSettings::default(),
            languages: LanguageSettings::default(),
            flow_bar: FlowBarSettings::default(),
            notifications: NotificationSettings::default(),
            app: AppSettings::default(),
            onboarding: OnboardingSettings::default(),
        }
    }
}

/// First-launch setup (ONB-01..08). Mirrors `Settings["onboarding"]` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OnboardingSettings {
    pub completed: bool,
    /// Where it resumes (ONB-07).
    pub step: OnboardingStep,
    /// The app version that last checked the permissions (ONB-08).
    pub version: String,
    /// Permissions seen granted, so losing one is noticed.
    pub granted: GrantedPermissions,
}

impl OnboardingSettings {
    /// Settings files from before onboarding existed belong to set-up installs.
    pub fn done() -> Self {
        Self {
            completed: true,
            step: OnboardingStep::Done,
            ..Self::default()
        }
    }

    /// Dictation shortcuts work once setup reaches Practice (ONB-07).
    pub fn shortcuts_active(&self) -> bool {
        self.completed || self.step >= OnboardingStep::Practice
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OnboardingStep {
    #[default]
    Welcome,
    Permissions,
    Microphone,
    Shortcut,
    Languages,
    Engine,
    Practice,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GrantedPermissions {
    pub microphone: bool,
    pub accessibility: bool,
}

/// The interface languages (LANG-06); "system" follows the Mac.
pub const APP_LANGUAGES: [&str; 6] = ["system", "en", "de", "es", "it", "pt"];

/// Settings › General's app language and Settings › System.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    /// One of [`APP_LANGUAGES`].
    pub language: String,
    /// Shrimp starts at login, in the background (APP-06).
    pub launch_at_login: bool,
    /// A Dock icon, or only the menu bar icon (APP-07).
    pub show_in_dock: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "system".into(),
            launch_at_login: false,
            show_in_dock: true,
        }
    }
}

/// Settings › Notifications (NOT-01): the categories that can be muted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NotificationSettings {
    pub suggestions: bool,
    pub announcements: bool,
    pub milestones: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            suggestions: true,
            announcements: true,
            milestones: true,
        }
    }
}

/// Settings › Experimental.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExperimentalSettings {
    /// Command Mode, off until the user turns it on (CMD-06).
    pub command_mode: bool,
    /// The one-time Command Mode intro has been shown.
    pub command_mode_intro_seen: bool,
    /// Ending a dictation with "press enter" presses Enter (CORE-16).
    pub press_enter_command: bool,
}

/// The Dictionary page's settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DictionarySettings {
    /// Words the user respells after a paste are added (DIC-07).
    pub auto_learn: bool,
}

impl Default for DictionarySettings {
    fn default() -> Self {
        Self { auto_learn: true }
    }
}

/// Settings › Vibe coding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VibeCodingSettings {
    /// File names said in Cursor's and Windsurf's chat become @-mentions (DEV-01).
    pub file_tagging: bool,
}

impl Default for VibeCodingSettings {
    fn default() -> Self {
        Self { file_tagging: true }
    }
}

/// How often one-off notices have been shown.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NoticeSettings {
    /// "Command mode is toggled off": at most 3 times, once a day.
    pub command_mode_off_count: u32,
    /// The local date it was last shown, YYYY-MM-DD.
    pub command_mode_off_day: String,
    /// When the Bluetooth microphone advice was last shown, in seconds since
    /// 1970; it shows at most once an hour (MIC-08).
    pub bluetooth_advice_at: u64,
    /// "Don't show again".
    pub bluetooth_advice_off: bool,
    /// "Speaking a different language?": the local date it was last shown, at most daily (LANG-04).
    pub language_card_day: String,
    pub language_card_off: bool,
    /// Formatting tips shown so far (FMT-13).
    pub formatting_tips: FormattingTips,
}

/// How many times each formatting tip was shown: one explanation, then up
/// to 3 reminders, at most one a day.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FormattingTips {
    pub list: u32,
    pub email: u32,
    pub backtrack: u32,
    /// The local date a tip was last shown, YYYY-MM-DD.
    pub day: String,
}

/// Data and privacy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PrivacySettings {
    /// Names and terms from the screen may reach the speech engine and the
    /// AI provider (CTX-04). Casing and spacing use the context either way.
    pub context_awareness: bool,
    /// How long transcripts are kept (HIS-07).
    pub history: HistoryStorage,
    /// Only local Whisper and Ollama run (PRIV-03).
    pub privacy_mode: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            context_awareness: true,
            history: HistoryStorage::Local,
            privacy_mode: false,
        }
    }
}

/// Data and privacy › Local data storage. Mirrors `HistoryStorage` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HistoryStorage {
    /// Transcripts stay until deleted; recordings 14 days.
    #[default]
    Local,
    /// Transcripts older than 24 hours are deleted, checked hourly.
    AutoDelete,
    /// Nothing is stored; History reads "History disabled".
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SttSettings {
    pub engine: SttEngineId,
    /// The Whisper model on this Mac (ENG-08).
    pub local_model: String,
    pub openai_model: String,
    pub groq_model: String,
}

impl Default for SttSettings {
    fn default() -> Self {
        Self {
            engine: SttEngineId::LocalWhisper,
            local_model: DEFAULT_WHISPER_MODEL.into(),
            openai_model: catalog::DEFAULT_OPENAI_STT_MODEL.into(),
            groq_model: catalog::DEFAULT_GROQ_STT_MODEL.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AiSettings {
    pub provider: LlmProviderId,
    pub cleanup: CleanupLevel,
    pub anthropic_model: String,
    pub openai_model: String,
    pub groq_model: String,
    /// Empty means the first installed model Shrimp recommends.
    pub ollama_model: String,
    pub ollama_url: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            provider: LlmProviderId::Anthropic,
            cleanup: CleanupLevel::Light,
            anthropic_model: catalog::default_model(LlmProviderId::Anthropic).into(),
            openai_model: catalog::default_model(LlmProviderId::OpenAi).into(),
            groq_model: catalog::default_model(LlmProviderId::Groq).into(),
            ollama_model: String::new(),
            ollama_url: catalog::DEFAULT_OLLAMA_URL.into(),
        }
    }
}

impl AiSettings {
    pub fn model(&self, provider: LlmProviderId) -> &str {
        match provider {
            LlmProviderId::Anthropic => &self.anthropic_model,
            LlmProviderId::OpenAi => &self.openai_model,
            LlmProviderId::Groq => &self.groq_model,
            LlmProviderId::Ollama => &self.ollama_model,
        }
    }
}

/// The style per app category (STY-02, STY-03). Every category starts Formal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StyleSettings {
    pub personal: Style,
    pub work: Style,
    pub email: Style,
    pub other: Style,
}

impl Default for StyleSettings {
    fn default() -> Self {
        Self {
            personal: Style::Formal,
            work: Style::Formal,
            email: Style::Formal,
            other: Style::Formal,
        }
    }
}

impl StyleSettings {
    pub fn for_category(&self, category: AppCategory) -> Style {
        match category {
            AppCategory::Personal => self.personal,
            AppCategory::Work => self.work,
            AppCategory::Email => self.email,
            AppCategory::Other => self.other,
        }
    }
}

impl Settings {
    /// The settings dictation runs with: Privacy mode turns every cloud
    /// engine and provider into local Whisper and Ollama (PRIV-03).
    pub fn effective(mut self) -> Self {
        if self.privacy.privacy_mode {
            self.stt.engine = SttEngineId::LocalWhisper;
            self.ai.provider = LlmProviderId::Ollama;
        }
        self
    }

    /// Moves values older versions stored elsewhere into place.
    pub fn normalize(&mut self) {
        self.shortcuts.normalize(&mut self.transforms);
    }

    /// Rejects values the UI should never send.
    pub fn validate(&self) -> Result<(), String> {
        for (category, style) in [
            (AppCategory::Personal, self.styles.personal),
            (AppCategory::Work, self.styles.work),
            (AppCategory::Email, self.styles.email),
            (AppCategory::Other, self.styles.other),
        ] {
            if !category.styles().contains(&style) {
                return Err(t!(
                    "error.settings.styleNotOffered",
                    style = format!("{style:?}"),
                    category = format!("{category:?}")
                ));
            }
        }
        let models = [
            &self.stt.openai_model,
            &self.stt.groq_model,
            &self.ai.anthropic_model,
            &self.ai.openai_model,
            &self.ai.groq_model,
        ];
        if models
            .iter()
            .any(|model| model.trim().is_empty() || model.len() > MAX_MODEL_LENGTH)
        {
            return Err(t!("error.settings.modelName", max = MAX_MODEL_LENGTH));
        }
        if self.ai.ollama_model.len() > MAX_MODEL_LENGTH {
            return Err(t!("error.settings.modelName", max = MAX_MODEL_LENGTH));
        }
        if find_whisper_model(&self.stt.local_model).is_none() {
            return Err(t!(
                "error.settings.unknownWhisperModel",
                model = self.stt.local_model
            ));
        }
        self.shortcuts.validate(&self.transforms)?;
        self.microphone.validate()?;
        self.media.validate()?;
        self.languages.validate()?;
        self.flow_bar.validate()?;
        if !APP_LANGUAGES.contains(&self.app.language.as_str()) {
            return Err(t!(
                "error.settings.unknownAppLanguage",
                language = self.app.language
            ));
        }
        self.transforms.validate()?;
        let url = self.ai.ollama_url.trim();
        if !(url.starts_with("http://") || url.starts_with("https://")) || url.len() > 200 {
            return Err(t!("error.settings.ollamaUrl"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_light_cleanup_and_formal_everywhere() {
        let settings = Settings::default();
        assert_eq!(settings.ai.cleanup, CleanupLevel::Light);
        assert_eq!(settings.ai.provider, LlmProviderId::Anthropic);
        assert_eq!(settings.stt.engine, SttEngineId::LocalWhisper);
        assert_eq!(settings.styles, StyleSettings::default());
        assert_eq!(settings.styles.personal, Style::Formal);
        settings.validate().unwrap();
    }

    #[test]
    fn serializes_in_camel_case_for_the_frontend() {
        let json = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(json["stt"]["engine"], "local-whisper");
        assert_eq!(json["ai"]["provider"], "anthropic");
        assert_eq!(json["ai"]["cleanup"], "light");
        assert_eq!(json["ai"]["ollamaUrl"], "http://localhost:11434");
        assert_eq!(json["styles"]["personal"], "formal");
        assert_eq!(json["privacy"]["contextAwareness"], true);
        assert_eq!(json["experimental"]["commandMode"], false);
        assert_eq!(json["shortcuts"]["bindings"], serde_json::json!({}));
    }

    #[test]
    fn shortcuts_must_parse() {
        let mut settings = Settings::default();
        settings
            .shortcuts
            .bindings
            .insert("pressEnter".into(), vec!["Opt+Return".into()]);
        settings.validate().unwrap();
        settings
            .shortcuts
            .bindings
            .insert("pressEnter".into(), vec!["Opt+Nothing".into()]);
        assert!(settings.validate().is_err());
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let settings: Settings =
            serde_json::from_value(serde_json::json!({"ai": {"cleanup": "medium"}})).unwrap();
        assert_eq!(settings.ai.cleanup, CleanupLevel::Medium);
        assert_eq!(settings.ai.provider, LlmProviderId::Anthropic);
        assert_eq!(settings.version, SETTINGS_VERSION);
    }

    #[test]
    fn styles_must_be_offered_by_their_category() {
        let mut settings = Settings::default();
        settings.styles.work = Style::VeryCasual;
        assert!(settings.validate().is_err());
        settings.styles.work = Style::Excited;
        settings.styles.personal = Style::VeryCasual;
        settings.validate().unwrap();
        settings.styles.personal = Style::Excited;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn privacy_mode_keeps_everything_local() {
        let mut settings = Settings::default();
        settings.stt.engine = SttEngineId::Groq;
        assert_eq!(
            settings.clone().effective().ai.provider,
            LlmProviderId::Anthropic
        );
        settings.privacy.privacy_mode = true;
        let effective = settings.effective();
        assert_eq!(effective.stt.engine, SttEngineId::LocalWhisper);
        assert_eq!(effective.ai.provider, LlmProviderId::Ollama);
    }

    #[test]
    fn shortcuts_wait_for_practice() {
        let mut onboarding = OnboardingSettings::default();
        assert!(!onboarding.shortcuts_active());
        onboarding.step = OnboardingStep::Engine;
        assert!(!onboarding.shortcuts_active());
        onboarding.step = OnboardingStep::Practice;
        assert!(onboarding.shortcuts_active());
        assert!(OnboardingSettings::done().shortcuts_active());
    }

    #[test]
    fn the_ollama_address_must_be_http() {
        let mut settings = Settings::default();
        settings.ai.ollama_url = "file:///etc/passwd".into();
        assert!(settings.validate().is_err());
    }
}
