//! Verify OTP (ported from `usecases/verify-otp/v1.ts`): lock check, RFC 6238 match with replay
//! guard, lockout after 5 failures. On success the caller unlocks the app until `expires_at_ms`
//! (the desktop has no session token; the unlock lives in the shell's memory).

use crate::domain::totp::{after_failure, lock_seconds_left, match_step};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::model::{TotpState, Versioned};
use crate::ports::{AuthStateStore, Clock};
use crate::util::base32;

/// What verify-otp needs.
pub struct AuthDeps<'a> {
    /// TOTP state.
    pub auth: &'a dyn AuthStateStore,
    /// Time.
    pub clock: &'a dyn Clock,
    /// The base32 TOTP secret baked in at build time.
    pub totp_secret: &'a str,
    /// Unlock lifetime in seconds.
    pub unlock_ttl_seconds: i64,
}

/// A successful unlock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unlock {
    /// When the unlock ends, epoch ms.
    pub expires_at_ms: i64,
}

fn locked(seconds: u64) -> AppError {
    AppError::new(ErrorCode::OtpLocked, "too many wrong codes; wait").with_retry_after(seconds)
}

fn fail(deps: &AuthDeps<'_>, stored: &Versioned<TotpState>, now: i64) -> AppError {
    let next = after_failure(&stored.value, now);
    if let Err(e) = deps.auth.record_failure(&next, stored.version) {
        return e;
    }
    match lock_seconds_left(&next, now) {
        0 => AppError::new(ErrorCode::OtpInvalid, "wrong code"),
        seconds => locked(seconds),
    }
}

/// Checks a six-digit code and unlocks.
///
/// # Errors
/// `OTP_LOCKED` (with the seconds left), `OTP_INVALID`, `OTP_REPLAYED`, `VERSION_CONFLICT`, and
/// `INTERNAL` when the secret is not base32 or the state store fails.
pub fn verify_otp(deps: &AuthDeps<'_>, code: &str) -> AppResult<Unlock> {
    let now = deps.clock.now_ms();
    let stored = deps.auth.get_auth_state()?;
    let seconds = lock_seconds_left(&stored.value, now);
    if seconds > 0 {
        return Err(locked(seconds));
    }
    let secret = base32::decode(deps.totp_secret)
        .ok_or_else(|| AppError::new(ErrorCode::Internal, "TOTP secret is not base32"))?;
    let Some(step) = match_step(&secret, code, now) else {
        return Err(fail(deps, &stored, now));
    };
    deps.auth.accept_step(step)?;
    Ok(Unlock {
        expires_at_ms: now + deps.unlock_ttl_seconds * 1000,
    })
}
