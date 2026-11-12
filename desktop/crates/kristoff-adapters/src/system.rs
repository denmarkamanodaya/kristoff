//! The system clock and the random id generator (ring `SystemRandom`). Ids and ULIDs are
//! monotonic within one millisecond, as in `backend/src/adapters/crypto-id-generator/v1.ts`.

use kristoff_core::domain::ids::{ULID_RANDOM_CHARS, format_id, format_ulid, increment};
use kristoff_core::ports::{Clock, IdGenerator};
use ring::rand::{SecureRandom, SystemRandom};
use std::sync::{Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

/// Reads the OS clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        let since = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
    }
}

#[derive(Debug, Default)]
struct Last {
    id_ms: i64,
    id_tail: u16,
    ulid_ms: i64,
    ulid_digits: [u8; ULID_RANDOM_CHARS],
}

/// Random, time-ordered ids from `clock`.
#[derive(Debug)]
pub struct RandomIdGenerator<C: Clock> {
    clock: C,
    rng: SystemRandom,
    last: Mutex<Last>,
}

impl<C: Clock> RandomIdGenerator<C> {
    /// A generator reading time from `clock`.
    pub fn new(clock: C) -> Self {
        Self {
            clock,
            rng: SystemRandom::new(),
            last: Mutex::new(Last::default()),
        }
    }

    /// Random bytes; zeros if the OS source fails (ids stay unique through the time part and
    /// the in-millisecond increment).
    fn random<const N: usize>(&self) -> [u8; N] {
        let mut bytes = [0u8; N];
        let _filled = self.rng.fill(&mut bytes);
        bytes
    }
}

impl<C: Clock> IdGenerator for RandomIdGenerator<C> {
    fn new_id(&self) -> String {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        let ms = self.clock.now_ms().max(last.id_ms);
        last.id_tail = if ms == last.id_ms {
            last.id_tail.wrapping_add(1)
        } else {
            u16::from_be_bytes(self.random::<2>()) & 0x7fff
        };
        last.id_ms = ms;
        format_id(ms, last.id_tail)
    }

    fn new_ulid(&self) -> String {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        let ms = self.clock.now_ms().max(last.ulid_ms);
        last.ulid_digits = if ms == last.ulid_ms {
            increment(&last.ulid_digits)
        } else {
            self.random::<ULID_RANDOM_CHARS>().map(|b| b % 32)
        };
        last.ulid_ms = ms;
        format_ulid(ms, &last.ulid_digits)
    }
}

#[cfg(test)]
mod tests {
    use super::{RandomIdGenerator, SystemClock};
    use kristoff_core::domain::ids::is_ulid;
    use kristoff_core::ports::{Clock, IdGenerator};
    use kristoff_core::util::text::is_hex_id;

    struct Frozen;
    impl Clock for Frozen {
        fn now_ms(&self) -> i64 {
            1_794_445_200_000
        }
    }

    #[test]
    fn ids_are_well_formed_and_strictly_increasing_within_one_millisecond() {
        let ids = RandomIdGenerator::new(Frozen);
        let a = ids.new_id();
        let b = ids.new_id();
        let (u, v) = (ids.new_ulid(), ids.new_ulid());

        assert!(is_hex_id(&a) && is_hex_id(&b) && b > a);
        assert!(a.starts_with("01a1cd44ca80"));
        assert!(is_ulid(&u) && is_ulid(&v) && v > u);
    }

    #[test]
    fn the_system_clock_is_after_2026() {
        assert!(SystemClock.now_ms() > 1_767_225_600_000);
    }
}
