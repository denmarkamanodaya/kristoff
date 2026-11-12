//! Shared setup for the command tests: a composition on a temp dir with a settable clock.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(dead_code)]

use kristoff_app::ai::AiPorts;
use kristoff_app::composition::Composition;
use kristoff_core::domain::model_request::FollowUps;
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
    /// An app whose model answers with the default script.
    pub fn new() -> Self {
        Self::answering(default_script(), FollowUps::default())
    }

    /// An app whose model plays `script` and suggests `follow_ups`; real lopdf splitter.
    pub fn answering(script: Script, follow_ups: FollowUps) -> Self {
        Self::answering_turns(vec![script], follow_ups)
    }

    /// The same, with one script per model turn (the last repeats).
    pub fn answering_turns(scripts: Vec<Script>, follow_ups: FollowUps) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let clock = Arc::new(FixedClock::at(START_MS));
        let remote = Arc::new(MemoryRemoteFiles::new());
        let log = Arc::new(MemoryLogger::default());
        let ai = AiPorts {
            model: Box::new(ScriptedModel::turns(scripts)),
            follow_ups: Box::new(FixedFollowUps::returning(follow_ups)),
            remote: Box::new(RemoteRef(Arc::clone(&remote))),
            splitter: Box::new(LopdfSplitter),
            log: log.clone(),
        };
        let core = Composition::with_ports(dir.path(), Box::new(Arc::clone(&clock)), ai).unwrap();
        Self {
            core,
            clock,
            dir,
            remote,
            log,
        }
    }
}
