//! RFC 6238 TOTP (SHA-1, 6 digits, 30 s step, window ±1 step) and the lockout rule (5 consecutive
//! failures lock every code for 15 minutes). Ported from `backend/src/domain/auth/verify-totp` and
//! `totp-lockout`. HMAC is deterministic, so it is allowed in the core.

use crate::model::TotpState;
use crate::util::text::constant_time_eq;
use ring::hmac;

/// Seconds per TOTP time step.
pub const TOTP_STEP_SECONDS: i64 = 30;
/// Digits in a login code.
const DIGITS: u32 = 6;
/// Steps accepted either side of the current one.
const WINDOW: i64 = 1;
/// Consecutive failures that trigger a lock.
const MAX_FAILURES: u32 = 5;
/// Lock length in milliseconds (15 minutes).
pub const LOCK_MS: i64 = 15 * 60 * 1000;

/// The HOTP code (RFC 4226) for one counter, zero-padded to `digits`.
#[must_use]
pub fn hotp(secret: &[u8], counter: u64, digits: u32) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, secret);
    let tag = hmac::sign(&key, &counter.to_be_bytes());
    let mac = tag.as_ref();
    let offset = usize::from(mac[mac.len() - 1] & 0x0f);
    let word = [
        mac[offset],
        mac[offset + 1],
        mac[offset + 2],
        mac[offset + 3],
    ];
    let binary = u32::from_be_bytes(word) & 0x7fff_ffff;
    let code = binary % 10u32.pow(digits);
    format!("{code:0width$}", width = digits as usize)
}

/// The time step that contains an instant: `floor(seconds / 30)`.
#[must_use]
pub fn step_at(now_ms: i64) -> i64 {
    now_ms.div_euclid(1000).div_euclid(TOTP_STEP_SECONDS)
}

/// Finds the step within ±1 of now whose code equals `code`, comparing every candidate in
/// constant time. `None` when the code is not six digits or no step in the window matches.
#[must_use]
pub fn match_step(secret: &[u8], code: &str, now_ms: i64) -> Option<i64> {
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let current = step_at(now_ms);
    let mut matched = None;
    for step in current - WINDOW..=current + WINDOW {
        let Ok(counter) = u64::try_from(step) else {
            continue;
        };
        let same = constant_time_eq(hotp(secret, counter, DIGITS).as_bytes(), code.as_bytes());
        if same && matched.is_none() {
            matched = Some(step);
        }
    }
    matched
}

/// Whole seconds left on a lock (rounded up), or 0 when not locked.
#[must_use]
pub fn lock_seconds_left(state: &TotpState, now_ms: i64) -> u64 {
    let left = state.locked_until_ms - now_ms;
    if left <= 0 {
        0
    } else {
        u64::try_from((left + 999) / 1000).unwrap_or(u64::MAX)
    }
}

/// The state after one wrong code: the fifth consecutive failure locks and resets the count.
#[must_use]
pub fn after_failure(state: &TotpState, now_ms: i64) -> TotpState {
    let fail_count = state.fail_count + 1;
    if fail_count >= MAX_FAILURES {
        TotpState {
            fail_count: 0,
            locked_until_ms: now_ms + LOCK_MS,
            ..*state
        }
    } else {
        TotpState {
            fail_count,
            ..*state
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{after_failure, hotp, lock_seconds_left, match_step, step_at};
    use crate::model::TotpState;

    const RFC_SECRET: &[u8] = b"12345678901234567890";
    const NOW: i64 = 1_800_000_000_000;

    fn counter(step: i64) -> u64 {
        u64::try_from(step).unwrap()
    }

    #[test]
    fn matches_the_rfc_6238_sha1_vectors() {
        let vectors = [
            (59, "94287082"),
            (1_111_111_109, "07081804"),
            (1_111_111_111, "14050471"),
            (1_234_567_890, "89005924"),
            (2_000_000_000, "69279037"),
            (20_000_000_000, "65353130"),
        ];
        for (seconds, expected) in vectors {
            assert_eq!(
                hotp(RFC_SECRET, counter(step_at(seconds * 1000)), 8),
                expected
            );
        }
    }

    #[test]
    fn accepts_the_current_step_and_one_either_side() {
        let now = 1_111_111_111_000;
        let current = step_at(now);
        for step in [current - 1, current, current + 1] {
            let code = hotp(RFC_SECRET, counter(step), 6);
            assert_eq!(match_step(RFC_SECRET, &code, now), Some(step));
        }
    }

    #[test]
    fn rejects_a_code_two_steps_away_and_non_six_digit_codes() {
        let now = 1_111_111_111_000;
        let old = hotp(RFC_SECRET, counter(step_at(now) - 2), 6);

        assert_eq!(match_step(RFC_SECRET, &old, now), None);
        assert_eq!(match_step(RFC_SECRET, "12345", now), None);
        assert_eq!(match_step(RFC_SECRET, "abcdef", now), None);
    }

    #[test]
    fn counts_the_first_four_failures_without_locking() {
        let four = (0..4).fold(TotpState::default(), |s, _| after_failure(&s, NOW));

        assert_eq!(
            four,
            TotpState {
                last_step: 0,
                fail_count: 4,
                locked_until_ms: 0
            }
        );
    }

    #[test]
    fn locks_for_fifteen_minutes_on_the_fifth_failure_and_resets_the_count() {
        let state = TotpState {
            last_step: 7,
            fail_count: 4,
            locked_until_ms: 0,
        };

        let next = after_failure(&state, NOW);

        assert_eq!(
            next,
            TotpState {
                last_step: 7,
                fail_count: 0,
                locked_until_ms: NOW + 900_000
            }
        );
    }

    #[test]
    fn reports_seconds_left_rounded_up_and_zero_once_passed() {
        let locked = TotpState {
            last_step: 0,
            fail_count: 0,
            locked_until_ms: NOW + 1500,
        };

        assert_eq!(lock_seconds_left(&locked, NOW), 2);
        assert_eq!(lock_seconds_left(&locked, NOW + 1500), 0);
    }
}
