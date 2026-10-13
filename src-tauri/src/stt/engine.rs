use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::audio::AudioClip;
use crate::t;

/// Mirrors `SttEngineId` in src/lib/ipc.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SttEngineId {
    LocalWhisper,
    #[serde(rename = "openai")]
    OpenAi,
    Groq,
}

impl SttEngineId {
    pub fn label(self) -> &'static str {
        match self {
            Self::LocalWhisper => "Whisper",
            Self::OpenAi => "OpenAI",
            Self::Groq => "Groq",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SttOptions {
    /// ISO 639-1 codes the user speaks. Empty means auto-detect any language;
    /// one code forces it; several restrict detection to those languages.
    pub languages: Vec<String>,
    /// Personal dictionary terms that bias recognition (Whisper's initial
    /// prompt, the cloud APIs' prompt or keyword fields).
    pub vocabulary: Vec<String>,
    /// Text that leads Whisper's prompt for a language, by code: it steers
    /// Chinese towards Simplified or Traditional characters (LANG-03).
    pub hints: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcript {
    pub text: String,
    /// Language the engine detected or was told to use, when it reports one.
    pub language: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SttError {
    #[error("{}", t!("error.stt.noSpeech"))]
    NoSpeech,
    /// The engine cannot run yet: model not downloaded, API key missing, offline.
    #[error("{}", t!("error.stt.unavailable", details = .0))]
    Unavailable(String),
    #[error("{}", t!("error.stt.failed", details = .0))]
    Failed(String),
    #[error("{}", t!("error.stt.cancelled"))]
    Cancelled,
}

/// One speech-to-text backend. Implementations must be cheap to call
/// repeatedly: load models once and reuse them across dictations.
#[async_trait]
pub trait SttEngine: Send + Sync {
    fn id(&self) -> SttEngineId;

    async fn transcribe(
        &self,
        clip: &AudioClip,
        options: &SttOptions,
    ) -> Result<Transcript, SttError>;

    /// The language most likely spoken and its probability, for engines that
    /// can tell (LANG-04).
    async fn detect_language(&self, _clip: &AudioClip) -> Option<(String, f32)> {
        None
    }
}
