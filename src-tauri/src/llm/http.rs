//! HTTP plumbing shared by the providers and cloud STT: one pooled client and
//! the mapping from transport and status errors to `LlmError`.

use std::time::Duration;

use serde_json::Value;

use super::LlmError;

const MAX_MESSAGE_CHARS: usize = 200;

/// One client for every provider, so connections (and TLS sessions) are
/// reused between dictations.
pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|error| {
            log::warn!("HTTP client setup failed ({error}); using the defaults");
            reqwest::Client::new()
        })
}

pub fn transport_error(error: &reqwest::Error) -> LlmError {
    if error.is_timeout() {
        LlmError::Timeout
    } else if error.is_connect() {
        LlmError::Offline(short(&error.to_string()))
    } else {
        LlmError::Failed(short(&error.to_string()))
    }
}

/// Sends a request and returns the JSON body of a successful response.
pub async fn send_json(request: reqwest::RequestBuilder) -> Result<Value, LlmError> {
    let response = request.send().await.map_err(|err| transport_error(&err))?;
    let status = response.status().as_u16();
    let body = response.text().await.map_err(|err| transport_error(&err))?;
    if !(200..300).contains(&status) {
        return Err(status_error(status, &body));
    }
    serde_json::from_str(&body)
        .map_err(|err| LlmError::Failed(format!("unexpected response: {err}")))
}

/// Maps an error response. Anthropic, OpenAI and Groq send
/// `{"error": {"message", "type", "code"}}`; Ollama sends `{"error": "..."}`.
pub fn status_error(status: u16, body: &str) -> LlmError {
    let json: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let error = &json["error"];
    let message = error["message"]
        .as_str()
        .or_else(|| error.as_str())
        .map_or_else(|| format!("HTTP {status}"), short);
    let kind = format!(
        "{} {}",
        error["type"].as_str().unwrap_or_default(),
        error["code"].as_str().unwrap_or_default()
    )
    .to_lowercase();
    let quota = ["quota", "credit", "spend_limit", "billing"]
        .iter()
        .any(|word| kind.contains(word) || message.to_lowercase().contains(&format!("{word} ")));
    match status {
        401 | 403 => LlmError::Unauthorized(message),
        402 => LlmError::Quota(message),
        429 if quota => LlmError::Quota(message),
        429 => LlmError::RateLimited(message),
        400 if message.to_lowercase().contains("credit balance") => LlmError::Quota(message),
        408 => LlmError::Timeout,
        404 | 498 | 500 | 502 | 503 | 504 | 529 => LlmError::Unavailable(message),
        _ => LlmError::Failed(message),
    }
}

fn short(text: &str) -> String {
    let mut short: String = text.chars().take(MAX_MESSAGE_CHARS).collect();
    if text.chars().count() > MAX_MESSAGE_CHARS {
        short.push('…');
    }
    short
}

/// A generous output budget: cleanup output is about as long as its input.
pub fn output_budget(text: &str) -> u32 {
    let words = text.split_whitespace().count() as u32;
    (words * 3 + 128).clamp(256, 16_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_statuses_to_errors() {
        let openai_quota = r#"{"error":{"message":"You have no credits remaining.","type":"insufficient_quota","param":null,"code":"credit_balance_exhausted"}}"#;
        assert_eq!(
            status_error(429, openai_quota),
            LlmError::Quota("You have no credits remaining.".into())
        );
        let groq_limit = r#"{"error":{"message":"Rate limit reached for model","type":"tokens"}}"#;
        assert!(matches!(
            status_error(429, groq_limit),
            LlmError::RateLimited(_)
        ));
        let anthropic = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        assert_eq!(
            status_error(401, anthropic),
            LlmError::Unauthorized("invalid x-api-key".into())
        );
        let overloaded =
            r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
        assert!(matches!(
            status_error(529, overloaded),
            LlmError::Unavailable(_)
        ));
        let ollama = r#"{"error":"model \"x\" not found, try pulling it first"}"#;
        assert!(
            matches!(status_error(404, ollama), LlmError::Unavailable(m) if m.contains("pulling"))
        );
        let low = r#"{"type":"error","error":{"type":"invalid_request_error","message":"Your credit balance is too low to access the Anthropic API."}}"#;
        assert!(matches!(status_error(400, low), LlmError::Quota(_)));
        assert_eq!(
            status_error(500, "<html>"),
            LlmError::Unavailable("HTTP 500".into())
        );
    }

    #[test]
    fn output_budget_scales_with_the_input() {
        assert_eq!(output_budget("short"), 256);
        assert_eq!(output_budget(&"word ".repeat(1000)), 3128);
        assert_eq!(output_budget(&"word ".repeat(100_000)), 16_000);
    }
}
