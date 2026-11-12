//! `World`: every fake wired together, with the use-case deps built from it.

use crate::fakes::{FakeProcessor, FixedClock, MemoryFiles, MemoryStore, SequenceIds};
use kristoff_core::ports::ProcessedDoc;
use kristoff_core::usecases::chats::ChatDeps;
use kristoff_core::usecases::docs::LibraryDeps;
use kristoff_core::usecases::projects::ProjectDeps;
use kristoff_core::usecases::upload::UploadDeps;

/// The instant every world starts at: 2026-11-12T01:00:00.000Z.
pub const START_MS: i64 = 1_794_445_200_000;

/// Every fake, wired together.
#[derive(Debug)]
pub struct World {
    /// Docs, chats, projects, usage and auth state.
    pub store: MemoryStore,
    /// PDF bytes.
    pub files: MemoryFiles,
    /// Time.
    pub clock: FixedClock,
    /// Ids.
    pub ids: SequenceIds,
    /// The doc processor (no page count, no parts unless replaced).
    pub processor: FakeProcessor,
}

impl Default for World {
    fn default() -> Self {
        Self {
            store: MemoryStore::new(),
            files: MemoryFiles::new(),
            clock: FixedClock::at(START_MS),
            ids: SequenceIds::default(),
            processor: FakeProcessor::returning(Ok(ProcessedDoc::default())),
        }
    }
}

impl World {
    /// A fresh world.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The library deps.
    #[must_use]
    pub fn library(&self) -> LibraryDeps<'_> {
        LibraryDeps {
            docs: &self.store,
            chats: &self.store,
            files: &self.files,
            clock: &self.clock,
            ids: &self.ids,
        }
    }

    /// The upload deps.
    #[must_use]
    pub fn upload(&self) -> UploadDeps<'_> {
        UploadDeps {
            docs: &self.store,
            files: &self.files,
            processor: &self.processor,
        }
    }

    /// The chat deps with an attach cap of 5.
    #[must_use]
    pub fn chats(&self) -> ChatDeps<'_> {
        ChatDeps {
            chats: &self.store,
            docs: &self.store,
            projects: &self.store,
            clock: &self.clock,
            ids: &self.ids,
            max_attached_docs: 5,
        }
    }

    /// The project deps.
    #[must_use]
    pub fn projects(&self) -> ProjectDeps<'_> {
        ProjectDeps {
            projects: &self.store,
            chats: &self.store,
            clock: &self.clock,
            ids: &self.ids,
        }
    }
}
