//! Anthropic Messages API (`POST /v1/messages`), raw HTTP: Anthropic has no
//! official Rust SDK.

use async_trait::async_trait;
use serde_json::{Value, json};

use super::http::send_json;
use super::{LlmError, LlmProvider, LlmProviderId, LlmRequest, LlmResponse};
use crate::t;

pub const BASE_URL: &str = "https://api.anthropic.com";
const VERSION: &str = "2023-06-01";

pub struct Anthropic {
    client: reqwest::Client,
    base_url: String,
    key: String,
    model: String,
}

impl Anthropic {
    pub fn new(client: reqwest::Client, base_url: &str, key: String, model: String) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_owned(),
            key,
            model,
        }
    }
}

/// The fastest thinking setting each model accepts: Haiku 5.5 can turn
/// thinking off, Sonnet 5.5 off with `between_tools`, Opus 5.5 only lower
/// its effort.
fn thinking(model: &str, body: &mut Value) {
    if model.starts_with("claude-haiku-5-5") {
        body["thinking"] = json!({"type": "disabled"});
    } else if model.starts_with("claude-sonnet-5-5") {
        body["thinking"] = json!({"type": "between_tools"});
        body["output_config"] = json!({"effort": "low"});
    } else if model.starts_with("claude-opus-5-5") {
        body["output_config"] = json!({"effort": "low"});
    }
}

pub fn body(model: &str, request: &LlmRequest) -> Value {
    let messages: Vec<Value> = request
        .messages
        .iter()
        .map(|message| json!({"role": message.role.as_str(), "content": message.content}))
        .collect();
    let mut body = json!({
        "model": model,
        "max_tokens": request.max_output_tokens,
        "system": request.system,
        "messages": messages,
    });
    thinking(model, &mut body);
    body
}

/// Joins the `text` blocks; thinking blocks may come first.
pub fn parse(response: &Value) -> Result<LlmResponse, LlmError> {
    if response["stop_reason"] == "refusal" {
        return Err(LlmError::Failed(t!("error.llm.declined")));
    }
    let blocks = response["content"]
        .as_array()
        .ok_or_else(|| LlmError::Failed(t!("error.llm.noContent")))?;
    let text: String = blocks
        .iter()
        .filter(|block| block["type"] == "text")
        .filter_map(|block| block["text"].as_str())
        .collect();
    Ok(LlmResponse { text })
}

#[async_trait]
impl LlmProvider for Anthropic {
    fn id(&self) -> LlmProviderId {
        LlmProviderId::Anthropic
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let response = send_json(
            self.client
                .post(format!("{}/v1/messages", self.base_url))
                .header("x-api-key", &self.key)
                .header("anthropic-version", VERSION)
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

    fn request() -> LlmRequest {
        LlmRequest::single(
            "Clean up.".into(),
            "<transcript>um hi</transcript>".into(),
            256,
        )
    }

    #[test]
    fn builds_a_messages_request_per_model() {
        let haiku = body("claude-haiku-5-5", &request());
        assert_eq!(
            haiku,
            json!({
                "model": "claude-haiku-5-5",
                "max_tokens": 256,
                "system": "Clean up.",
                "messages": [{"role": "user", "content": "<transcript>um hi</transcript>"}],
                "thinking": {"type": "disabled"}
            })
        );
        let sonnet = body("claude-sonnet-5-5", &request());
        assert_eq!(sonnet["thinking"], json!({"type": "between_tools"}));
        assert_eq!(sonnet["output_config"], json!({"effort": "low"}));
        let opus = body("claude-opus-5-5", &request());
        assert_eq!(opus["output_config"], json!({"effort": "low"}));
        assert!(opus.get("thinking").is_none());
        assert!(opus.get("temperature").is_none());
    }

    #[test]
    fn reads_text_blocks_after_thinking() {
        let response = json!({
            "id": "msg_01", "type": "message", "role": "assistant", "model": "claude-opus-5-5",
            "content": [
                {"type": "thinking", "thinking": "", "signature": "abc"},
                {"type": "text", "text": "Hi."}
            ],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 20, "output_tokens": 3}
        });
        assert_eq!(parse(&response).unwrap().text, "Hi.");
        let refusal = json!({"content": [], "stop_reason": "refusal"});
        assert!(parse(&refusal).is_err());
    }
}
