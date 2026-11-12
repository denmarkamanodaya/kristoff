//! Verify OTP: ported from `backend/src/usecases/verify-otp/v1.test.ts`.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::domain::totp::{hotp, step_at};
use kristoff_core::error::ErrorCode;
use kristoff_core::ports::AuthStateStore;
use kristoff_core::usecases::auth::{AuthDeps, Unlock, verify_otp};
use kristoff_testkit::contract::assert_code;
use kristoff_testkit::fakes::{FixedClock, MemoryStore};
use kristoff_testkit::world::START_MS;

/// RFC 6238 secret "12345678901234567890" in base32.
const TOTP: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
const RAW: &[u8] = b"12345678901234567890";

struct Setup {
    store: MemoryStore,
    clock: FixedClock,
}

impl Setup {
    fn new() -> Self {
        Self {
            store: MemoryStore::new(),
            clock: FixedClock::at(START_MS),
        }
    }

    fn deps(&self) -> AuthDeps<'_> {
        self.deps_with(TOTP)
    }

    fn deps_with<'a>(&'a self, secret: &'a str) -> AuthDeps<'a> {
        AuthDeps {
            auth: &self.store,
            clock: &self.clock,
            totp_secret: secret,
            unlock_ttl_seconds: 43_200,
        }
    }

    fn code_at_step_offset(&self, offset: i64) -> String {
        let step = step_at(kristoff_core::ports::Clock::now_ms(&self.clock)) + offset;
        hotp(RAW, u64::try_from(step).unwrap(), 6)
    }
}

#[test]
fn unlocks_for_the_configured_lifetime() {
    let s = Setup::new();

    let unlock = verify_otp(&s.deps(), &s.code_at_step_offset(0)).unwrap();

    assert_eq!(
        unlock,
        Unlock {
            expires_at_ms: START_MS + 43_200_000
        }
    );
}

#[test]
fn refuses_the_same_code_a_second_time() {
    let s = Setup::new();
    let code = s.code_at_step_offset(0);
    verify_otp(&s.deps(), &code).unwrap();

    assert_code(verify_otp(&s.deps(), &code), ErrorCode::OtpReplayed);
}

#[test]
fn refuses_an_older_code_after_a_newer_one_was_used() {
    let s = Setup::new();
    verify_otp(&s.deps(), &s.code_at_step_offset(0)).unwrap();

    assert_code(
        verify_otp(&s.deps(), &s.code_at_step_offset(-1)),
        ErrorCode::OtpReplayed,
    );
}

#[test]
fn answers_otp_invalid_for_a_wrong_code_and_counts_the_failure() {
    let s = Setup::new();

    assert_code(verify_otp(&s.deps(), "000000"), ErrorCode::OtpInvalid);
    assert_eq!(s.store.get_auth_state().unwrap().value.fail_count, 1);
}

#[test]
fn locks_on_the_fifth_wrong_code_for_900_seconds_then_refuses_the_right_code() {
    let s = Setup::new();
    for _ in 0..4 {
        assert_code(verify_otp(&s.deps(), "000000"), ErrorCode::OtpInvalid);
    }

    let fifth = verify_otp(&s.deps(), "000000").unwrap_err();
    let right = verify_otp(&s.deps(), &s.code_at_step_offset(0)).unwrap_err();

    assert_eq!(
        (fifth.code, fifth.retry_after_seconds),
        (ErrorCode::OtpLocked, Some(900))
    );
    assert_eq!(right.code, ErrorCode::OtpLocked);
}

#[test]
fn accepts_the_right_code_once_the_lock_has_passed_and_resets_the_count() {
    let s = Setup::new();
    for _ in 0..5 {
        let _ = verify_otp(&s.deps(), "000000");
    }
    s.clock.advance(15 * 60 * 1000);

    verify_otp(&s.deps(), &s.code_at_step_offset(0)).unwrap();

    let state = s.store.get_auth_state().unwrap().value;
    assert_eq!((state.fail_count, state.locked_until_ms), (0, 0));
}

#[test]
fn fails_closed_when_the_secret_is_not_base32() {
    let s = Setup::new();

    assert_code(
        verify_otp(&s.deps_with("!!!"), "123456"),
        ErrorCode::Internal,
    );
    assert_code(verify_otp(&s.deps_with(""), "123456"), ErrorCode::Internal);
}
