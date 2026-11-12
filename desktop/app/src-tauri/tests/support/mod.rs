//! Shared setup for the command tests: a composition on a temp dir with a settable clock and the
//! RFC 6238 test secret.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(dead_code)]

use kristoff_app::ai::AiPorts;
use kristoff_app::commands::session;
use kristoff_app::composition::Composition;
use kristoff_core::domain::model_request::FollowUps;
use kristoff_core::domain::totp::{hotp, step_at};
use kristoff_core::error::AppResult;
use kristoff_core::ports::RemoteFiles;
use kristoff_pdf::LopdfSplitter;
use kristoff_testkit::fakes::{
    FixedClock, FixedFollowUps, MemoryLogger, MemoryRemoteFiles, Script, ScriptedModel,
};
use kristoff_testkit::world::{START_MS, default_script};
use std::sync::Arc;

/// The test's handle on the in-memory Files API the composition uses.
struct RemoteRef(Arc<MemoryRemoteFiles>);

impl RemoteFiles for RemoteRef {
    fn upload(&self, name: &str, pdf: &[u8]) -> AppResult<String> {
        self.0.upload(name, pdf)
    }
    fn delete(&self, ids: &[String]) {
        self.0.delete(ids);
    }
}

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
    /// The Files API it uploads to.
    pub remote: Arc<MemoryRemoteFiles>,
    /// Its warning log.
    pub log: Arc<MemoryLogger>,
}

impl TestApp {
    /// A locked app whose model answers with the default script.
    pub fn locked() -> Self {
        Self::answering(default_script(), FollowUps::default())
    }

    /// A locked app whose model plays `script` and suggests `follow_ups`; real lopdf splitter.
    pub fn answering(script: Script, follow_ups: FollowUps) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let clock = Arc::new(FixedClock::at(START_MS));
        let remote = Arc::new(MemoryRemoteFiles::new());
        let log = Arc::new(MemoryLogger::default());
        let ai = AiPorts {
            model: Box::new(ScriptedModel::new(script)),
            follow_ups: Box::new(FixedFollowUps::returning(follow_ups)),
            remote: Box::new(RemoteRef(Arc::clone(&remote))),
            splitter: Box::new(LopdfSplitter),
            log: log.clone(),
        };
        let core =
            Composition::with_ports(dir.path(), TOTP, Box::new(Arc::clone(&clock)), ai).unwrap();
        Self {
            core,
            clock,
            dir,
            remote,
            log,
        }
    }

    /// An app unlocked with a live code.
    pub fn unlocked() -> Self {
        Self::locked().unlock()
    }

    /// The same app, unlocked with a live code.
    pub fn unlock(self) -> Self {
        session::verify(&self.core, &self.code()).unwrap();
        self
    }

    /// The live TOTP code now.
    pub fn code(&self) -> String {
        let step = step_at(kristoff_core::ports::Clock::now_ms(&self.clock));
        hotp(b"12345678901234567890", u64::try_from(step).unwrap(), 6)
    }
}
