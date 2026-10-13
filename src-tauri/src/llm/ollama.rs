//! Local Ollama (`POST /api/chat`). Nothing leaves the device.

use async_trait::async_trait;
use serde_json::{Value, json};

use super::http::send_json;
use super::{LlmError, LlmProvider, LlmProviderId, LlmRequest, LlmResponse};
use crate::t;

/// Keeps the model in memory between dictations.
const KEEP_ALIVE: &str = "30m";

pub struct Ollama {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl Ollama {
    pub fn new(client: reqwest::Client, base_url: &str, model: String) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_owned(),
            model,
        }
    }
}

/// The installed models (`GET /api/tags`).
pub async fn installed_models(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<Vec<String>, LlmError> {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let response = send_json(client.get(url)).await.map_err(not_running)?;
    Ok(parse_tags(&response))
}

pub fn parse_tags(response: &Value) -> Vec<String> {
    response["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|model| model["name"].as_str().map(str::to_owned))
        .collect()
}

/// A refused connection means Ollama isn't running.
fn not_running(error: LlmError) -> LlmError {
    match error {
        LlmError::Offline(_) => LlmError::Unavailable(t!("error.llm.ollamaNotRunning")),
        error => error,
    }
}

pub fn body(model: &str, request: &LlmRequest) -> Value {
    let mut messages = vec![json!({"role": "system", "content": request.system})];
    messages.extend(
        request
            .messages
            .iter()
            .map(|message| json!({"role": message.role.as_str(), "content": message.content})),
    );
    json!({
        "model": model,
        "messages": messages,
        "stream": false,
        // Ignored by models without thinking; turns it off for the rest.
        "think": false,
        "keep_alive": KEEP_ALIVE,
        "options": {
            "temperature": if request.varied { 0.8 } else { 0.0 },
            "num_predict": request.max_output_tokens,
        },
    })
}

pub fn parse(response: &Value) -> Result<LlmResponse, LlmError> {
    let text = response["message"]["content"]
        .as_str()
        .ok_or_else(|| LlmError::Failed(t!("error.llm.noMessage")))?;
    Ok(LlmResponse { text: text.into() })
}

#[async_trait]
impl LlmProvider for Ollama {
    fn id(&self) -> LlmProviderId {
        LlmProviderId::Ollama
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let response = send_json(
            self.client
                .post(format!("{}/api/chat", self.base_url))
                .json(&body(&self.model, request)),
        )
        .await
        .map_err(not_running)?;
        parse(&response)
    }

    /// Loads the model into memory while the user speaks.
    async fn warm_up(&self) {
        let request = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&json!({"model": self.model, "keep_alive": KEEP_ALIVE}));
        let _ = request.send().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_varied_request_samples_with_temperature() {
        let mut request = LlmRequest::single("Clean up.".into(), "um hi".into(), 256);
        assert_eq!(body("qwen3.5:4b", &request)["options"]["temperature"], 0.0);
        request.varied = true;
        assert_eq!(body("qwen3.5:4b", &request)["options"]["temperature"], 0.8);
    }

    #[test]
    fn builds_a_chat_request_with_thinking_off() {
        let request = LlmRequest::single("Clean up.".into(), "um hi".into(), 256);
        let body = body("gemma3:4b", &request);
        assert_eq!(body["think"], false);
        assert_eq!(body["stream"], false);
        assert_eq!(body["keep_alive"], "30m");
        assert_eq!(body["options"]["num_predict"], 256);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["content"], "um hi");
    }

    #[test]
    fn reads_the_message_and_the_model_list() {
        let response = json!({
            "model": "gemma3:4b", "created_at": "2026-10-13T12:00:00Z",
            "message": {"role": "assistant", "content": "Hi."},
            "done": true, "done_reason": "stop", "total_duration": 1000, "eval_count": 3
        });
        assert_eq!(parse(&response).unwrap().text, "Hi.");
        let tags = json!({"models": [{"name": "gemma3:4b", "model": "gemma3:4b", "size": 1}]});
        assert_eq!(parse_tags(&tags), ["gemma3:4b"]);
    }

    #[test]
    fn a_refused_connection_reads_as_not_running() {
        assert_eq!(
            not_running(LlmError::Offline("refused".into())),
            LlmError::Unavailable("Ollama isn't running".into())
        );
    }
}
