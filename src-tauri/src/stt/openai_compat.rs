//! Cloud speech-to-text through the OpenAI-style transcription endpoint,
//! used by OpenAI and Groq (ENG-05).

use async_trait::async_trait;
use reqwest::multipart::{Form, Part};
use serde_json::Value;

use super::whisper_local::vocabulary_prompt;
use super::{SttEngine, SttEngineId, SttError, SttOptions, Transcript};
use crate::audio::{AudioClip, wav_encode};
use crate::llm::{LlmError, http};

pub const OPENAI_URL: &str = "https://api.openai.com/v1/audio/transcriptions";
pub const GROQ_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
/// Both services cap uploads at 25 MB (about 13 minutes of 16 kHz WAV).
const MAX_UPLOAD_BYTES: usize = 25 * 1024 * 1024;
/// Groq's `prompt` holds at most 224 tokens.
const MAX_PROMPT_CHARS: usize = 600;

pub struct CloudStt {
    id: SttEngineId,
    client: reqwest::Client,
    url: String,
    key: String,
    model: String,
}

impl CloudStt {
    pub fn new(
        id: SttEngineId,
        client: reqwest::Client,
        url: &str,
        key: String,
        model: String,
    ) -> Self {
        Self {
            id,
            client,
            url: url.to_owned(),
            key,
            model,
        }
    }
}

/// The text fields of the upload. `gpt-transcribe` takes `languages[]` and
/// `keywords[]`; Whisper models take one `language` and a `prompt`.
pub fn fields(model: &str, options: &SttOptions) -> Vec<(String, String)> {
    let mut fields = vec![
        ("model".to_owned(), model.to_owned()),
        ("response_format".to_owned(), "json".to_owned()),
    ];
    if model.starts_with("gpt-transcribe") {
        for language in &options.languages {
            fields.push(("languages[]".into(), language.clone()));
        }
        for keyword in &options.vocabulary {
            let clean: String = keyword
                .chars()
                .filter(|c| !matches!(c, '<' | '>' | '\r' | '\n'))
                .collect();
            if !clean.trim().is_empty() {
                fields.push(("keywords[]".into(), clean));
            }
        }
    } else {
        if let [language] = options.languages.as_slice() {
            fields.push(("language".into(), language.clone()));
        }
        let prompt: String = vocabulary_prompt(&options.vocabulary)
            .chars()
            .take(MAX_PROMPT_CHARS)
            .collect();
        if !prompt.is_empty() {
            fields.push(("prompt".into(), prompt));
        }
    }
    fields
}

pub fn parse(response: &Value) -> Result<Transcript, SttError> {
    let text = response["text"]
        .as_str()
        .ok_or_else(|| SttError::Failed("no text in the response".into()))?
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        return Err(SttError::NoSpeech);
    }
    let language = response["language"]
        .as_str()
        .or_else(|| response["languages"][0]["code"].as_str())
        .map(str::to_owned);
    Ok(Transcript { text, language })
}

fn stt_error(name: &str, error: LlmError) -> SttError {
    match error {
        LlmError::MissingKey | LlmError::Unauthorized(_) => {
            SttError::Unavailable(format!("{name} rejected your API key."))
        }
        LlmError::Quota(_) => {
            SttError::Unavailable(format!("Your {name} account is out of credits."))
        }
        LlmError::RateLimited(_) => SttError::Unavailable(format!("{name} rate limit reached.")),
        LlmError::Offline(_) => SttError::Unavailable(format!("Can't reach {name}.")),
        LlmError::Timeout => SttError::Unavailable(format!("{name} took too long.")),
        other => SttError::Failed(format!("{name}: {other}")),
    }
}

#[async_trait]
impl SttEngine for CloudStt {
    fn id(&self) -> SttEngineId {
        self.id
    }

    async fn transcribe(
        &self,
        clip: &AudioClip,
        options: &SttOptions,
    ) -> Result<Transcript, SttError> {
        let name = self.id.label();
        let wav = wav_encode::wav_bytes(clip).map_err(|err| SttError::Failed(err.to_string()))?;
        if wav.len() > MAX_UPLOAD_BYTES {
            return Err(SttError::Failed(format!(
                "{name} accepts up to 25 MB of audio; this dictation is longer."
            )));
        }
        let part = Part::bytes(wav)
            .file_name("dictation.wav")
            .mime_str("audio/wav")
            .map_err(|err| SttError::Failed(err.to_string()))?;
        let form = fields(&self.model, options)
            .into_iter()
            .fold(Form::new().part("file", part), |form, (name, value)| {
                form.text(name, value)
            });
        let request = self
            .client
            .post(&self.url)
            .bearer_auth(&self.key)
            .multipart(form);
        let response = http::send_json(request)
            .await
            .map_err(|err| stt_error(name, err))?;
        parse(&response)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn options() -> SttOptions {
        SttOptions {
            languages: vec!["en".into()],
            vocabulary: vec!["Shrimp".into(), "Tauri<x>".into()],
        }
    }

    #[test]
    fn gpt_transcribe_gets_languages_and_keywords() {
        assert_eq!(
            fields("gpt-transcribe", &options()),
            [
                ("model".to_owned(), "gpt-transcribe".to_owned()),
                ("response_format".into(), "json".into()),
                ("languages[]".into(), "en".into()),
                ("keywords[]".into(), "Shrimp".into()),
                ("keywords[]".into(), "Taurix".into()),
            ]
        );
    }

    #[test]
    fn whisper_models_get_a_language_and_a_prompt() {
        assert_eq!(
            fields("whisper-large-v3-turbo", &options()),
            [
                ("model".to_owned(), "whisper-large-v3-turbo".to_owned()),
                ("response_format".into(), "json".into()),
                ("language".into(), "en".into()),
                ("prompt".into(), "Shrimp, Tauri<x>.".into()),
            ]
        );
        let auto = SttOptions::default();
        assert_eq!(fields("whisper-large-v3", &auto).len(), 2);
    }

    #[test]
    fn reads_the_transcript() {
        let groq = json!({"text": " Hello  there. ", "x_groq": {"id": "req_1"}});
        assert_eq!(parse(&groq).unwrap().text, "Hello there.");
        let openai = json!({"text": "Hola.", "languages": [{"code": "es"}], "usage": {}});
        assert_eq!(parse(&openai).unwrap().language.as_deref(), Some("es"));
        assert!(matches!(
            parse(&json!({"text": "  "})),
            Err(SttError::NoSpeech)
        ));
    }

    #[test]
    fn errors_read_clearly() {
        let quota = stt_error("OpenAI", LlmError::Quota("no credits".into()));
        assert_eq!(
            quota.to_string(),
            "speech-to-text unavailable: Your OpenAI account is out of credits."
        );
    }
}
