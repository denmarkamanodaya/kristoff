//! The one error-code list and the typed error every port and use case returns.
//! Same list as `backend/src/contracts/errors.ts` `ERROR_CODES` (a contracts test pins it).

use std::fmt;

/// Every error code the app can produce, wire-visible or internal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// The request is malformed or out of bounds.
    ValidationFailed,
    /// The TOTP code is wrong.
    OtpInvalid,
    /// The TOTP code (or a later one) was already used.
    OtpReplayed,
    /// Too many wrong codes; every code is refused until the lock ends.
    OtpLocked,
    /// The app is locked: no valid unlock.
    Unauthenticated,
    /// No such document, or it is not ready.
    DocNotFound,
    /// No such chat.
    ChatNotFound,
    /// No such project.
    ProjectNotFound,
    /// The project cap is reached.
    ProjectLimitReached,
    /// The uploaded bytes are not an acceptable PDF upload.
    UploadInvalid,
    /// The daily question cap is reached.
    DailyCapReached,
    /// Asking is switched off.
    AiDisabled,
    /// An outside service failed.
    UpstreamUnavailable,
    /// Internal only: an optimistic-lock write lost a race. Sent as `INTERNAL`.
    VersionConflict,
    /// Anything else.
    Internal,
}

impl ErrorCode {
    /// Every code, in the order of `ERROR_CODES`.
    pub const ALL: [Self; 15] = [
        Self::ValidationFailed,
        Self::OtpInvalid,
        Self::OtpReplayed,
        Self::OtpLocked,
        Self::Unauthenticated,
        Self::DocNotFound,
        Self::ChatNotFound,
        Self::ProjectNotFound,
        Self::ProjectLimitReached,
        Self::UploadInvalid,
        Self::DailyCapReached,
        Self::AiDisabled,
        Self::UpstreamUnavailable,
        Self::VersionConflict,
        Self::Internal,
    ];

    /// The `UPPER_SNAKE` name callers switch on.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ValidationFailed => "VALIDATION_FAILED",
            Self::OtpInvalid => "OTP_INVALID",
            Self::OtpReplayed => "OTP_REPLAYED",
            Self::OtpLocked => "OTP_LOCKED",
            Self::Unauthenticated => "UNAUTHENTICATED",
            Self::DocNotFound => "DOC_NOT_FOUND",
            Self::ChatNotFound => "CHAT_NOT_FOUND",
            Self::ProjectNotFound => "PROJECT_NOT_FOUND",
            Self::ProjectLimitReached => "PROJECT_LIMIT_REACHED",
            Self::UploadInvalid => "UPLOAD_INVALID",
            Self::DailyCapReached => "DAILY_CAP_REACHED",
            Self::AiDisabled => "AI_DISABLED",
            Self::UpstreamUnavailable => "UPSTREAM_UNAVAILABLE",
            Self::VersionConflict => "VERSION_CONFLICT",
            Self::Internal => "INTERNAL",
        }
    }
}

/// A typed failure: a stable code, a human message and an optional wait in seconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppError {
    /// The code callers switch on.
    pub code: ErrorCode,
    /// For humans; may change.
    pub message: String,
    /// Seconds the client should wait before trying again (lock end, next day), if any.
    pub retry_after_seconds: Option<u64>,
}

impl AppError {
    /// An error without a wait.
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retry_after_seconds: None,
        }
    }

    /// The same error with a wait in seconds.
    #[must_use]
    pub fn with_retry_after(mut self, seconds: u64) -> Self {
        self.retry_after_seconds = Some(seconds);
        self
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for AppError {}

/// The result every port and use case returns.
pub type AppResult<T> = Result<T, AppError>;
