//! OpenAI Responses API (`POST /v1/responses`).

use async_trait::async_trait;
use serde_json::{Value, json};

use super::http::send_json;
use super::{LlmError, LlmProvider, LlmProviderId, LlmRequest, LlmResponse};

pub const BASE_URL: &str = "https://api.openai.com";

pub struct OpenAi {
    client: reqwest::Client,
    base_url: String,
    key: String,
    model: String,
}

impl OpenAi {
    pub fn new(client: reqwest::Client, base_url: &str, key: String, model: String) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_owned(),
            key,
            model,
        }
    }
}

/// GPT-6 Luna, GPT-5.6 Luna and GPT-5.4 accept `effort: "none"`; GPT-6
/// Astra and Sol start at `low`; GPT-4.1 has no reasoning setting.
fn reasoning_effort(model: &str) -> Option<&'static str> {
    if model.starts_with("gpt-4") {
        None
    } else if model.starts_with("gpt-6-astra") || model.starts_with("gpt-6.1-sol") {
        Some("low")
    } else if model.starts_with("gpt-5") || model.starts_with("gpt-6") {
        Some("none")
    } else {
        None
    }
}

pub fn body(model: &str, request: &LlmRequest) -> Value {
    let input: Vec<Value> = request
        .messages
        .iter()
        .map(|message| json!({"role": message.role.as_str(), "content": message.content}))
        .collect();
    let mut body = json!({
        "model": model,
        "instructions": request.system,
        "input": input,
        "max_output_tokens": request.max_output_tokens,
        "store": false,
    });
    if let Some(effort) = reasoning_effort(model) {
        body["reasoning"] = json!({"effort": effort});
    }
    body
}

/// The text of the `message` items' `output_text` parts; reasoning items may
/// come first. (`output_text` at the top level is an SDK convenience.)
pub fn parse(response: &Value) -> Result<LlmResponse, LlmError> {
    let output = response["output"]
        .as_array()
        .ok_or_else(|| LlmError::Failed("no output in the response".into()))?;
    let mut text = String::new();
    for item in output.iter().filter(|item| item["type"] == "message") {
        for part in item["content"].as_array().into_iter().flatten() {
            match part["type"].as_str() {
                Some("output_text") => text.push_str(part["text"].as_str().unwrap_or_default()),
                Some("refusal") => return Err(LlmError::Failed("the model declined".into())),
                _ => {}
            }
        }
    }
    Ok(LlmResponse { text })
}

#[async_trait]
impl LlmProvider for OpenAi {
    fn id(&self) -> LlmProviderId {
        LlmProviderId::OpenAi
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let response = send_json(
            self.client
                .post(format!("{}/v1/responses", self.base_url))
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

    fn request() -> LlmRequest {
        LlmRequest::single(
            "Clean up.".into(),
            "<transcript>um hi</transcript>".into(),
            256,
        )
    }

    #[test]
    fn builds_a_responses_request() {
        assert_eq!(
            body("gpt-6-luna", &request()),
            json!({
                "model": "gpt-6-luna",
                "instructions": "Clean up.",
                "input": [{"role": "user", "content": "<transcript>um hi</transcript>"}],
                "max_output_tokens": 256,
                "store": false,
                "reasoning": {"effort": "none"}
            })
        );
        assert_eq!(
            body("gpt-6.1-sol", &request())["reasoning"]["effort"],
            "low"
        );
        assert!(body("gpt-4.1-mini", &request()).get("reasoning").is_none());
    }

    #[test]
    fn reads_output_text_after_reasoning_items() {
        let response = json!({
            "id": "resp_1", "object": "response", "status": "completed",
            "output": [
                {"type": "reasoning", "id": "rs_1", "summary": []},
                {"type": "message", "id": "msg_1", "role": "assistant", "status": "completed",
                 "content": [{"type": "output_text", "text": "Hi.", "annotations": []}]}
            ],
            "usage": {"input_tokens": 20, "output_tokens": 3}
        });
        assert_eq!(parse(&response).unwrap().text, "Hi.");
        let refusal = json!({"output": [{"type": "message", "content": [{"type": "refusal", "refusal": "No."}]}]});
        assert!(parse(&refusal).is_err());
    }
}
