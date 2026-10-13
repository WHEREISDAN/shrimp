//! The typed settings. Every section has defaults, so files written by older
//! versions (or missing fields) still load.

use serde::{Deserialize, Serialize};

use crate::edit::{AppCategory, CleanupLevel, Style};
use crate::llm::{LlmProviderId, catalog};
use crate::stt::SttEngineId;

pub const SETTINGS_VERSION: u32 = 1;
const MAX_MODEL_LENGTH: usize = 120;

/// Mirrors `Settings` in src/lib/ipc.ts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub stt: SttSettings,
    pub ai: AiSettings,
    pub styles: StyleSettings,
    pub privacy: PrivacySettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            stt: SttSettings::default(),
            ai: AiSettings::default(),
            styles: StyleSettings::default(),
            privacy: PrivacySettings::default(),
        }
    }
}

/// Data and privacy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PrivacySettings {
    /// Names and terms from the screen may reach the speech engine and the
    /// AI provider (CTX-04). Casing and spacing use the context either way.
    pub context_awareness: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            context_awareness: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SttSettings {
    pub engine: SttEngineId,
    pub openai_model: String,
    pub groq_model: String,
}

impl Default for SttSettings {
    fn default() -> Self {
        Self {
            engine: SttEngineId::LocalWhisper,
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
    /// Rejects values the UI should never send.
    pub fn validate(&self) -> Result<(), String> {
        for (category, style) in [
            (AppCategory::Personal, self.styles.personal),
            (AppCategory::Work, self.styles.work),
            (AppCategory::Email, self.styles.email),
            (AppCategory::Other, self.styles.other),
        ] {
            if !category.styles().contains(&style) {
                return Err(format!("{style:?} is not offered for {category:?}"));
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
            return Err("Model names must be 1 to 120 characters.".into());
        }
        if self.ai.ollama_model.len() > MAX_MODEL_LENGTH {
            return Err("Model names must be 1 to 120 characters.".into());
        }
        let url = self.ai.ollama_url.trim();
        if !(url.starts_with("http://") || url.starts_with("https://")) || url.len() > 200 {
            return Err("The Ollama address must start with http:// or https://.".into());
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
    fn the_ollama_address_must_be_http() {
        let mut settings = Settings::default();
        settings.ai.ollama_url = "file:///etc/passwd".into();
        assert!(settings.validate().is_err());
    }
}
