//! The IPC error envelope `{code, message, retryAfter}`. `VERSION_CONFLICT` is internal and
//! leaves as `INTERNAL`; messages are cut to 300 characters (the web envelope's bound).

use kristoff_core::error::{AppError, ErrorCode};
use serde::{Deserialize, Serialize};

/// Longest message on the wire.
const MAX_MESSAGE: usize = 300;

/// What every failed command returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ErrorDto {
    /// `UPPER_SNAKE` code from `ERROR_CODES`.
    pub code: String,
    /// For humans.
    pub message: String,
    /// Seconds to wait before trying again, or null.
    pub retry_after: Option<u64>,
    /// A short local id for this failure (a ULID), when the command layer set one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl ErrorDto {
    /// The same error tagged with `request_id`.
    #[must_use]
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

impl From<&AppError> for ErrorDto {
    fn from(e: &AppError) -> Self {
        let code = match e.code {
            ErrorCode::VersionConflict => ErrorCode::Internal,
            code => code,
        };
        Self {
            code: code.as_str().to_owned(),
            message: e.message.chars().take(MAX_MESSAGE).collect(),
            retry_after: e.retry_after_seconds,
            request_id: None,
        }
    }
}

impl From<AppError> for ErrorDto {
    fn from(e: AppError) -> Self {
        Self::from(&e)
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorDto;
    use kristoff_core::error::{AppError, ErrorCode};

    #[test]
    fn keeps_the_code_message_and_wait() {
        let e = AppError::new(ErrorCode::OtpLocked, "wait").with_retry_after(900);

        let dto = ErrorDto::from(&e);

        assert_eq!(
            dto,
            ErrorDto {
                code: "OTP_LOCKED".into(),
                message: "wait".into(),
                retry_after: Some(900),
                request_id: None
            }
        );
    }

    #[test]
    fn sends_a_version_conflict_as_internal_and_cuts_long_messages() {
        let e = AppError::new(ErrorCode::VersionConflict, "x".repeat(400));

        let dto = ErrorDto::from(e);

        assert_eq!(
            (dto.code.as_str(), dto.message.len(), dto.retry_after),
            ("INTERNAL", 300, None)
        );
    }
}
