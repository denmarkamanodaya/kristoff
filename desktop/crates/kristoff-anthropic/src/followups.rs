//! `AnthropicFollowUps`: one non-streaming Haiku call with JSON-schema output.
//!
//! Ported from `backend/src/adapters/anthropic-follow-up-suggester/v1.ts`. Model output is
//! untrusted (stack/ai-features.md): it is used only on `end_turn`, and only when it is exactly
//! `{"followUps": [...]}` with at most 5 strings of 1–120 UTF-16 units after trimming (the Zod
//! schema's bounds). Any failure gives no questions; it never becomes an error.

use crate::error::{AnthropicError, ErrorCode};
use crate::http::{self, ApiConfig};
use crate::request::follow_up_body;
use crate::sse::usage_of;
use kristoff_contracts::model_config::FOLLOW_UP_TIMEOUT_MS;
use kristoff_core::domain::model_request::FollowUps;
use kristoff_core::limits::{MAX_FOLLOW_UP, MAX_FOLLOW_UPS};
use kristoff_core::model::Usage;
use kristoff_core::util::text::utf16_len;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

/// Suggestions plus why there are none, for the one warning log line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Suggestion {
    /// The follow-ups (possibly none) and their usage.
    pub follow_ups: FollowUps,
    /// `call_failed` or `invalid_output` when there are none because of a failure.
    pub failure: Option<&'static str>,
}

/// Suggests follow-up questions.
#[derive(Clone)]
pub struct AnthropicFollowUps {
    client: Client,
    config: ApiConfig,
}

impl AnthropicFollowUps {
    /// Wraps the shared client (see [`crate::build_client`]).
    #[must_use]
    pub fn new(client: Client, config: ApiConfig) -> Self {
        Self { client, config }
    }

    /// Asks Haiku for follow-ups to `question` given the start of the answer.
    pub async fn suggest(&self, question: &str, answer_excerpt: &str) -> Suggestion {
        match self.call(question, answer_excerpt).await {
            Ok(message) => from_message(&message),
            Err(_) => Suggestion {
                failure: Some("call_failed"),
                ..Suggestion::default()
            },
        }
    }

    async fn call(&self, question: &str, answer_excerpt: &str) -> Result<Value, AnthropicError> {
        let body = follow_up_body(question, answer_excerpt);
        let bytes =
            serde_json::to_vec(&body).map_err(|_| AnthropicError::new(ErrorCode::LocalIo))?;
        let timeout = Duration::from_millis(FOLLOW_UP_TIMEOUT_MS);
        let make = || self.config.post_json(&self.client, &bytes).timeout(timeout);
        let response = http::send(make, None).await?;
        let raw = response
            .bytes()
            .await
            .map_err(|e| AnthropicError::from_reqwest(&e))?;
        serde_json::from_slice(&raw)
            .map_err(|_| AnthropicError::new(ErrorCode::UpstreamInvalidResponse))
    }
}

/// Turns a Messages API reply into follow-ups. Pure; exported for tests.
#[must_use]
pub fn from_message(message: &Value) -> Suggestion {
    let usage: Option<Usage> = Some(usage_of(&message["usage"], Usage::default()));
    match questions_of(message) {
        Some(questions) => Suggestion {
            follow_ups: FollowUps { questions, usage },
            failure: None,
        },
        None => Suggestion {
            follow_ups: FollowUps {
                questions: Vec::new(),
                usage,
            },
            failure: Some("invalid_output"),
        },
    }
}

/// The validated questions, or `None` when the reply is not trusted.
#[must_use]
pub fn questions_of(message: &Value) -> Option<Vec<String>> {
    // A cut-off or refused reply is not trusted, even when its text happens to parse.
    if message["stop_reason"] != "end_turn" {
        return None;
    }
    let content = message["content"].as_array()?;
    let text = content.iter().find(|b| b["type"] == "text")?["text"].as_str()?;
    let parsed: Value = serde_json::from_str(text).ok()?;
    let object = parsed.as_object()?;
    if object.len() != 1 {
        return None;
    }
    let items = object.get("followUps")?.as_array()?;
    if items.len() > MAX_FOLLOW_UPS {
        return None;
    }
    items.iter().map(valid_question).collect()
}

fn valid_question(item: &Value) -> Option<String> {
    let trimmed = item.as_str()?.trim();
    (1..=MAX_FOLLOW_UP)
        .contains(&utf16_len(trimmed))
        .then(|| trimmed.to_owned())
}
