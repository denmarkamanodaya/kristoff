//! `AskEvent`: one message on the `ask` command's channel (docs/contract.md "Desktop IPC"):
//! `{ event, data }` with exactly the SSE event names and data of "Stream".

use crate::shapes::{CitationDto, MessageDto, UsageDto};
use kristoff_core::error::AppError;
use serde::Serialize;

/// The `error` event's inner object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamErrorDto {
    /// `UPPER_SNAKE` code.
    pub code: String,
    /// For humans.
    pub message: String,
    /// The local request id.
    pub request_id: String,
}

/// The `error` event's data: `{ error: { code, message, requestId } }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StreamErrorData {
    /// The failure.
    pub error: StreamErrorDto,
}

/// One channel message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "lowercase")]
pub enum AskEvent {
    /// A piece of answer text: `{ text }`.
    Text {
        /// The text.
        text: String,
    },
    /// A citation: `Citation`.
    Citation(CitationDto),
    /// The follow-ups: `{ followUps }`.
    Followups {
        /// Suggested next questions.
        #[serde(rename = "followUps")]
        follow_ups: Vec<String>,
    },
    /// The stored answer and the call's usage: `{ message, usage }` (always the last on success).
    Done {
        /// The stored assistant message.
        message: MessageDto,
        /// Usage of the answer call.
        usage: UsageDto,
    },
    /// A failure after the stream opened (always the last on failure).
    Error(StreamErrorData),
}

impl AskEvent {
    /// The `error` event for `e`, with the command's request id. `VERSION_CONFLICT` leaves as
    /// `INTERNAL`, as in every error envelope.
    #[must_use]
    pub fn error(e: &AppError, request_id: String) -> Self {
        let dto = crate::error::ErrorDto::from(e);
        Self::Error(StreamErrorData {
            error: StreamErrorDto {
                code: dto.code,
                message: dto.message,
                request_id,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::AskEvent;
    use kristoff_core::error::{AppError, ErrorCode};
    use serde_json::json;

    #[test]
    fn events_serialise_as_event_and_data() {
        let text = serde_json::to_value(AskEvent::Text { text: "Hi".into() }).unwrap();
        let fu = serde_json::to_value(AskEvent::Followups {
            follow_ups: vec!["Q?".into()],
        })
        .unwrap();
        let err = AskEvent::error(
            &AppError::new(ErrorCode::UpstreamUnavailable, "down"),
            "r1".into(),
        );

        assert_eq!(text, json!({ "event": "text", "data": { "text": "Hi" } }));
        assert_eq!(
            fu,
            json!({ "event": "followups", "data": { "followUps": ["Q?"] } })
        );
        assert_eq!(
            serde_json::to_value(err).unwrap(),
            json!({ "event": "error", "data": { "error": { "code": "UPSTREAM_UNAVAILABLE", "message": "down", "requestId": "r1" } } })
        );
    }
}
