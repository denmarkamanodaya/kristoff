//! The typed error every adapter in this crate returns. No vendor error crosses it.

use std::fmt;

/// Stable codes the core switches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Network failure, timeout, 408/429/5xx after the retry, or an `error` event mid-stream.
    UpstreamUnavailable,
    /// The API refused the request (4xx other than 401/408/429): a bug or a bad document.
    UpstreamRejected,
    /// The API rejected the key (HTTP 401): missing, revoked or wrong (Phase 8).
    KeyRejected,
    /// The response could not be read (bad JSON, stream cut before `message_delta`).
    UpstreamInvalidResponse,
    /// The ask was cancelled through its [`crate::CancelToken`].
    Cancelled,
    /// A local file could not be read.
    LocalIo,
}

impl ErrorCode {
    /// The `UPPER_SNAKE` wire code.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UpstreamUnavailable => "UPSTREAM_UNAVAILABLE",
            Self::UpstreamRejected => "UPSTREAM_REJECTED",
            Self::KeyRejected => "KEY_INVALID",
            Self::UpstreamInvalidResponse => "UPSTREAM_INVALID_RESPONSE",
            Self::Cancelled => "CANCELLED",
            Self::LocalIo => "INTERNAL",
        }
    }
}

/// An adapter failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnthropicError {
    /// The stable code.
    pub code: ErrorCode,
    /// HTTP status, when one was received.
    pub status: Option<u16>,
    /// The API's `error.type` (or `timeout` / `connect`), for logs only.
    pub error_type: Option<String>,
}

impl AnthropicError {
    pub(crate) fn new(code: ErrorCode) -> Self {
        Self {
            code,
            status: None,
            error_type: None,
        }
    }

    pub(crate) fn cancelled() -> Self {
        Self::new(ErrorCode::Cancelled)
    }

    pub(crate) fn with_type(code: ErrorCode, error_type: &str) -> Self {
        Self {
            code,
            status: None,
            error_type: Some(error_type.to_owned()),
        }
    }

    pub(crate) fn from_reqwest(e: &reqwest::Error) -> Self {
        let kind = if e.is_timeout() {
            "timeout"
        } else if e.is_connect() {
            "connect"
        } else if e.is_body() || e.is_decode() {
            "body"
        } else {
            "request"
        };
        Self::with_type(ErrorCode::UpstreamUnavailable, kind)
    }
}

impl fmt::Display for AnthropicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code.as_str())?;
        if let Some(status) = self.status {
            write!(f, " (HTTP {status})")?;
        }
        if let Some(t) = &self.error_type {
            write!(f, " [{t}]")?;
        }
        Ok(())
    }
}

impl std::error::Error for AnthropicError {}
