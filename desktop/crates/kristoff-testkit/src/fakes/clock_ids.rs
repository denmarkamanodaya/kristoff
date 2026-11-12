//! A settable clock and deterministic, time-ordered ids.

use kristoff_core::domain::ids::{ULID_RANDOM_CHARS, format_id, format_ulid};
use kristoff_core::ports::{Clock, IdGenerator};
use std::sync::atomic::{AtomicI64, Ordering};

/// A clock that stays where it is put.
#[derive(Debug)]
pub struct FixedClock(AtomicI64);

impl FixedClock {
    /// A clock at `now_ms`.
    #[must_use]
    pub fn at(now_ms: i64) -> Self {
        Self(AtomicI64::new(now_ms))
    }

    /// Moves the clock forward by `ms`.
    pub fn advance(&self, ms: i64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
}

impl Clock for FixedClock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }
}

/// Ids from a counter: each call is one millisecond after the last, so ids sort by call order.
#[derive(Debug, Default)]
pub struct SequenceIds(AtomicI64);

const BASE_MS: i64 = 1_794_445_200_000;

impl IdGenerator for SequenceIds {
    fn new_id(&self) -> String {
        let n = self.0.fetch_add(1, Ordering::SeqCst);
        format_id(BASE_MS + n, u16::try_from(n % 0x8000).unwrap_or(0))
    }

    fn new_ulid(&self) -> String {
        let n = self.0.fetch_add(1, Ordering::SeqCst);
        let mut digits = [0u8; ULID_RANDOM_CHARS];
        digits[ULID_RANDOM_CHARS - 1] = u8::try_from(n % 32).unwrap_or(0);
        format_ulid(BASE_MS + n, &digits)
    }
}
