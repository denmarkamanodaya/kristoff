//! Shared setup for the command tests: a composition on a temp dir with a settable clock and the
//! RFC 6238 test secret.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(dead_code)]

use kristoff_app::commands::session;
use kristoff_app::composition::Composition;
use kristoff_core::domain::totp::{hotp, step_at};
use kristoff_testkit::fakes::FixedClock;
use kristoff_testkit::world::START_MS;
use std::sync::Arc;

/// RFC 6238 secret "12345678901234567890" in base32.
pub const TOTP: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

/// A composition on a temp dir with a settable clock.
pub struct TestApp {
    /// The composition under test.
    pub core: Composition,
    /// Its clock.
    pub clock: Arc<FixedClock>,
    /// Its app data dir (kept alive with the app).
    pub dir: tempfile::TempDir,
}

impl TestApp {
    /// A locked app.
    pub fn locked() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let clock = Arc::new(FixedClock::at(START_MS));
        let core = Composition::with_clock(dir.path(), TOTP, Box::new(Arc::clone(&clock))).unwrap();
        Self { core, clock, dir }
    }

    /// An app unlocked with a live code.
    pub fn unlocked() -> Self {
        let a = Self::locked();
        session::verify(&a.core, &a.code()).unwrap();
        a
    }

    /// The live TOTP code now.
    pub fn code(&self) -> String {
        let step = step_at(kristoff_core::ports::Clock::now_ms(&self.clock));
        hotp(b"12345678901234567890", u64::try_from(step).unwrap(), 6)
    }
}
