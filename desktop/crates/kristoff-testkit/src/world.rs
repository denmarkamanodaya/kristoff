//! `World`: every fake wired together, with the use-case deps built from it.

use crate::fakes::{
    FakeSplitter, FixedClock, FixedFollowUps, MemoryFiles, MemoryRemoteFiles, MemoryStore, Script,
    ScriptedModel, SequenceIds,
};
use kristoff_core::domain::model_request::{AnswerEnd, FollowUps, ModelEvent};
use kristoff_core::domain::parts::SplitLimits;
use kristoff_core::model::Usage;
use kristoff_core::usecases::ask::{AskConfig, AskDeps};
use kristoff_core::usecases::chats::ChatDeps;
use kristoff_core::usecases::docs::LibraryDeps;
use kristoff_core::usecases::processing::ProcessingDeps;
use kristoff_core::usecases::projects::ProjectDeps;
use kristoff_core::usecases::upload::UploadDeps;

/// The instant every world starts at: 2026-11-12T01:00:00.000Z.
pub const START_MS: i64 = 1_794_445_200_000;

/// The models the ask deps allow, first = default.
pub const MODEL_IDS: [&str; 2] = ["claude-sonnet-5-5", "claude-haiku-5-5"];

/// Usage of the default scripted answer.
pub const ANSWER_USAGE: Usage = Usage {
    input: 100,
    output: 20,
    cache_read: 5,
    cache_write: 7,
};

/// The default script: one text piece, then `end_turn`.
#[must_use]
pub fn default_script() -> Script {
    Script {
        events: vec![ModelEvent::Text("Answer.".into())],
        end: Ok(AnswerEnd {
            usage: ANSWER_USAGE,
            stop_reason: Some("end_turn".into()),
            ..AnswerEnd::default()
        }),
    }
}

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
    /// Splits [`crate::fakes::fake_pdf`] files.
    pub splitter: FakeSplitter,
    /// The Files API.
    pub remote: MemoryRemoteFiles,
    /// The answer model ([`default_script`] unless replaced).
    pub model: ScriptedModel,
    /// Follow-ups (none unless replaced).
    pub follow_ups: FixedFollowUps,
    /// Split limits for processing (the production ones unless replaced).
    pub limits: SplitLimits,
    /// Questions per day for ask.
    pub daily_cap: u32,
    /// The law index ask may search (none unless set: no tools are sent then).
    pub law: Option<crate::fakes::MemoryLawSearch>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            store: MemoryStore::new(),
            files: MemoryFiles::new(),
            clock: FixedClock::at(START_MS),
            ids: SequenceIds::default(),
            splitter: FakeSplitter,
            remote: MemoryRemoteFiles::new(),
            model: ScriptedModel::new(default_script()),
            follow_ups: FixedFollowUps::returning(FollowUps::default()),
            limits: SplitLimits::CLAUDE,
            daily_cap: 100,
            law: None,
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
            remote: &self.remote,
        }
    }

    /// The processing deps.
    #[must_use]
    pub fn processing(&self) -> ProcessingDeps<'_> {
        ProcessingDeps {
            docs: &self.store,
            files: &self.files,
            splitter: &self.splitter,
            remote: &self.remote,
            limits: self.limits,
        }
    }

    /// The ask deps: [`MODEL_IDS`], 4096 max tokens, 20 history turns.
    #[must_use]
    pub fn ask(&self) -> AskDeps<'_> {
        AskDeps {
            chats: &self.store,
            docs: &self.store,
            model: &self.model,
            follow_ups: &self.follow_ups,
            law: self
                .law
                .as_ref()
                .map(|l| l as &dyn kristoff_core::ports::LawSearch),
            meter: &self.store,
            ids: &self.ids,
            clock: &self.clock,
            config: AskConfig {
                model_ids: &MODEL_IDS,
                max_tokens: 4096,
                max_history_turns: 20,
                daily_question_cap: self.daily_cap,
            },
        }
    }

    /// The upload deps.
    #[must_use]
    pub fn upload(&self) -> UploadDeps<'_> {
        UploadDeps {
            docs: &self.store,
            files: &self.files,
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
