//! `AnthropicModel`: one streaming `POST /v1/messages` per call.

use crate::cancel::CancelToken;
use crate::error::{AnthropicError, ErrorCode};
use crate::http::{self, ApiConfig};
use crate::sse::{SseParser, StreamEvent};
use kristoff_core::model::Usage;
use reqwest::{Client, Response};
use serde_json::Value;

/// How a finished stream ended.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamOutcome {
    /// Final usage.
    pub usage: Usage,
    /// `end_turn`, `max_tokens`, `tool_use`, `refusal`, ...
    pub stop_reason: Option<String>,
    /// The assistant content blocks as produced, for a tool round (D-11).
    pub content: Vec<Value>,
}

/// Streams answers from the Messages API.
#[derive(Clone)]
pub struct AnthropicModel {
    client: Client,
    config: ApiConfig,
}

impl AnthropicModel {
    /// Wraps the shared client (see [`crate::build_client`]).
    #[must_use]
    pub fn new(client: Client, config: ApiConfig) -> Self {
        Self { client, config }
    }

    /// Sends `body` (from [`crate::request::messages_body`]) and calls `on_event` for every
    /// `TextDelta`, `CitationDelta`, `ToolUse` and `MessageDelta`, in stream order.
    ///
    /// # Errors
    /// `CANCELLED` when `cancel` fires (events already delivered stay delivered);
    /// `UPSTREAM_UNAVAILABLE` / `UPSTREAM_REJECTED` for HTTP failures and `error` events;
    /// `UPSTREAM_INVALID_RESPONSE` when the stream ends before its `message_delta`.
    pub async fn stream(
        &self,
        body: &Value,
        cancel: &CancelToken,
        on_event: &mut dyn FnMut(&StreamEvent),
    ) -> Result<StreamOutcome, AnthropicError> {
        let bytes =
            serde_json::to_vec(body).map_err(|_| AnthropicError::new(ErrorCode::LocalIo))?;
        let make = || self.config.post_json(&self.client, &bytes);
        let response = http::send(make, Some(cancel)).await?;
        read_stream(response, cancel, on_event).await
    }
}

async fn read_stream(
    mut response: Response,
    cancel: &CancelToken,
    on_event: &mut dyn FnMut(&StreamEvent),
) -> Result<StreamOutcome, AnthropicError> {
    let mut parser = SseParser::new();
    let mut last: Option<(Usage, Option<String>)> = None;
    loop {
        let chunk = http::cancellable(response.chunk(), Some(cancel))
            .await?
            .map_err(|e| AnthropicError::from_reqwest(&e))?;
        let is_end = chunk.is_none();
        let events = match chunk {
            Some(bytes) => parser.feed(&bytes),
            None => parser.finish(),
        };
        for event in &events {
            if let StreamEvent::Error { error_type, .. } = event {
                return Err(AnthropicError::with_type(
                    stream_error_code(error_type),
                    error_type,
                ));
            }
            if let StreamEvent::MessageDelta { usage, stop_reason } = event {
                last = Some((*usage, stop_reason.clone()));
            }
            on_event(event);
        }
        if is_end {
            break;
        }
    }
    let (usage, stop_reason) = last.ok_or_else(|| {
        AnthropicError::with_type(ErrorCode::UpstreamInvalidResponse, "stream_cut")
    })?;
    Ok(StreamOutcome {
        usage,
        stop_reason,
        content: parser.assistant_content(),
    })
}

/// An API `error` event means the upstream failed; a payload we could not read means the stream
/// itself is bad (for example cut inside a frame).
fn stream_error_code(error_type: &str) -> ErrorCode {
    match error_type {
        "invalid_event" | "invalid_tool_input" => ErrorCode::UpstreamInvalidResponse,
        _ => ErrorCode::UpstreamUnavailable,
    }
}
