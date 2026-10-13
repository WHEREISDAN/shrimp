//! Groq chat completions (OpenAI-compatible, `POST /openai/v1/chat/completions`).

use async_trait::async_trait;
use serde_json::{Value, json};

use super::http::send_json;
use super::{LlmError, LlmProvider, LlmProviderId, LlmRequest, LlmResponse};
use crate::t;

pub const BASE_URL: &str = "https://api.groq.com/openai/v1";

pub struct Groq {
    client: reqwest::Client,
    base_url: String,
    key: String,
    model: String,
}

impl Groq {
    pub fn new(client: reqwest::Client, base_url: &str, key: String, model: String) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_owned(),
            key,
            model,
        }
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
    let mut body = json!({
        "model": model,
        "messages": messages,
        "max_completion_tokens": request.max_output_tokens,
        "stream": false,
    });
    // GPT-OSS can't turn reasoning off; `low` is its fastest, and the
    // reasoning text is left out of the response.
    if model.starts_with("openai/gpt-oss") {
        body["reasoning_effort"] = json!("low");
        body["include_reasoning"] = json!(false);
    } else if model.starts_with("qwen/") {
        body["reasoning_effort"] = json!("none");
    }
    body
}

pub fn parse(response: &Value) -> Result<LlmResponse, LlmError> {
    let text = response["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| LlmError::Failed(t!("error.llm.noMessage")))?;
    Ok(LlmResponse { text: text.into() })
}

#[async_trait]
impl LlmProvider for Groq {
    fn id(&self) -> LlmProviderId {
        LlmProviderId::Groq
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let response = send_json(
            self.client
                .post(format!("{}/chat/completions", self.base_url))
                .bearer_auth(&self.key)
                .json(&body(&self.model, request)),
        )
        .await?;
        parse(&response)
    }

    async fn warm_up(&self) {
        let _ = self.client.head(&self.base_url).send().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_chat_request_without_reasoning_text() {
        let request = LlmRequest::single("Clean up.".into(), "um hi".into(), 256);
        assert_eq!(
            body("openai/gpt-oss-20b", &request),
            json!({
                "model": "openai/gpt-oss-20b",
                "messages": [
                    {"role": "system", "content": "Clean up."},
                    {"role": "user", "content": "um hi"}
                ],
                "max_completion_tokens": 256,
                "stream": false,
                "reasoning_effort": "low",
                "include_reasoning": false
            })
        );
        assert_eq!(
            body("qwen/qwen3.8-27b", &request)["reasoning_effort"],
            "none"
        );
    }

    #[test]
    fn reads_the_first_choice() {
        let response = json!({
            "id": "chatcmpl-1", "object": "chat.completion", "model": "openai/gpt-oss-20b",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "Hi."}, "finish_reason": "stop"}],
            "usage": {"queue_time": 0.01, "prompt_tokens": 20, "completion_tokens": 3, "total_time": 0.05},
            "x_groq": {"id": "req_1"}
        });
        assert_eq!(parse(&response).unwrap().text, "Hi.");
        assert!(parse(&json!({"choices": []})).is_err());
    }
}
