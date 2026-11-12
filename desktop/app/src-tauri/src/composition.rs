//! The composition root: builds every adapter once, binds them to the use cases' deps, and holds
//! the running asks. The only place that reads configuration
//! (`kristoff_contracts::config`, `model_config`) and the baked secrets.

use crate::ai::AiPorts;
use crate::asks::Asks;
use kristoff_adapters::disk_file_store::DiskFileStore;
use kristoff_adapters::sqlite::SqliteStore;
use kristoff_adapters::system::{RandomIdGenerator, SystemClock};
use kristoff_contracts::config::{DAILY_QUESTION_CAP, MAX_ATTACHED_DOCS, MODEL_IDS, MODEL_LABELS};
use kristoff_contracts::model_config::MAX_TOKENS;
use kristoff_core::domain::parts::SplitLimits;
use kristoff_core::error::AppResult;
use kristoff_core::limits::MAX_HISTORY_TURNS;
use kristoff_core::model::Doc;
use kristoff_core::ports::{Clock, DocStore, FileStore, IdGenerator, LawSearch};
use kristoff_core::usecases::ask::{AskConfig, AskDeps};
use kristoff_core::usecases::chats::ChatDeps;
use kristoff_core::usecases::docs::LibraryDeps;
use kristoff_core::usecases::processing::{self, ProcessingDeps};
use kristoff_core::usecases::projects::ProjectDeps;
use kristoff_core::usecases::settings::ConfigSource;
use kristoff_core::usecases::upload::UploadDeps;
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Everything the commands need, built once at start.
pub struct Composition {
    store: SqliteStore,
    files: DiskFileStore,
    ai: AiPorts,
    clock: Box<dyn Clock>,
    ids: Box<dyn IdGenerator>,
    /// The running asks and their cancel flags.
    pub asks: Asks,
    /// The PH law index, installed at start and warmed on a background thread.
    law: OnceLock<Box<dyn LawSearch>>,
    /// The background warm-up of `law` has finished (successfully or not).
    law_warmed: AtomicBool,
}

impl Composition {
    /// Opens the app data under `data_dir` (`kristoff.db`, `docs/`) with the system clock and
    /// the Anthropic adapters on `anthropic_key`.
    ///
    /// # Errors
    /// `INTERNAL` when the database, the docs folder or the Claude client cannot be opened.
    pub fn open(data_dir: &Path, anthropic_key: &str) -> AppResult<Self> {
        let ai = AiPorts::anthropic(anthropic_key)?;
        Self::with_ports(data_dir, Box::new(SystemClock), ai)
    }

    /// The same with an injected clock and AI ports (tests).
    ///
    /// # Errors
    /// `INTERNAL` when the database or the docs folder cannot be opened.
    pub fn with_ports(data_dir: &Path, clock: Box<dyn Clock>, ai: AiPorts) -> AppResult<Self> {
        let files = DiskFileStore::open(&data_dir.join("docs"))?;
        Ok(Self {
            store: SqliteStore::open(&data_dir.join("kristoff.db"))?,
            files,
            ai,
            clock,
            ids: Box::new(RandomIdGenerator::new(SystemClock)),
            asks: Asks::default(),
            law: OnceLock::new(),
            law_warmed: AtomicBool::new(false),
        })
    }

    /// Makes `law` the law index at once (not warmed: a search before the warm-up ends waits for
    /// it); a second install is ignored.
    pub fn install_law(&self, law: Box<dyn LawSearch>) {
        let _ = self.law.set(law);
    }

    /// Warms the installed law index (a no-op without one), then records that nothing is left to
    /// load, even when the warm-up failed, so the launch screen never waits forever.
    ///
    /// # Errors
    /// The warm-up failure (searches will then fail too).
    pub fn warm_law(&self) -> AppResult<()> {
        let warmed = self.law_search().map_or(Ok(()), LawSearch::warm);
        self.law_warmed.store(true, Ordering::SeqCst);
        warmed
    }

    /// Whether the background warm-up of the law index has finished.
    #[must_use]
    pub fn law_warmed(&self) -> bool {
        self.law_warmed.load(Ordering::SeqCst)
    }

    /// The law index, once installed; `None` when the build has no law data.
    #[must_use]
    pub fn law_search(&self) -> Option<&dyn LawSearch> {
        self.law.get().map(AsRef::as_ref)
    }

    /// Epoch ms now.
    #[must_use]
    pub fn now_ms(&self) -> i64 {
        self.clock.now_ms()
    }

    /// A fresh local id for a failed command.
    #[must_use]
    pub fn request_id(&self) -> String {
        self.ids.new_ulid()
    }

    /// The doc and file stores the viewer reads.
    #[must_use]
    pub fn viewer(&self) -> (&dyn DocStore, &dyn FileStore) {
        (&self.store, &self.files)
    }

    /// The library deps.
    #[must_use]
    pub fn library(&self) -> LibraryDeps<'_> {
        LibraryDeps {
            docs: &self.store,
            chats: &self.store,
            files: &self.files,
            clock: self.clock.as_ref(),
            ids: self.ids.as_ref(),
            remote: self.ai.remote.as_ref(),
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

    /// The processing deps with the Claude per-document limits.
    #[must_use]
    pub fn processing(&self) -> ProcessingDeps<'_> {
        ProcessingDeps {
            docs: &self.store,
            files: &self.files,
            splitter: self.ai.splitter.as_ref(),
            remote: self.ai.remote.as_ref(),
            limits: SplitLimits::CLAUDE,
        }
    }

    /// Processes one confirmed doc (split, upload, ready or failed). Runs on a blocking thread.
    ///
    /// # Errors
    /// The failure that stopped it; the doc is then `failed`.
    pub fn process_doc(&self, doc_id: &str) -> AppResult<Doc> {
        processing::process_doc(&self.processing(), doc_id)
    }

    /// Records a warning (never prompts, answers, titles or keys).
    pub fn warn(&self, event: &str, reason: &str) {
        self.ai.log.warn(event, reason);
    }

    /// The docs to process again at start (left in `processing`, or ready from a Phase-1 build).
    ///
    /// # Errors
    /// Store failures.
    pub fn docs_to_resume(&self) -> AppResult<Vec<String>> {
        processing::docs_to_resume(&self.store)
    }

    /// The ask deps.
    #[must_use]
    pub fn ask(&self) -> AskDeps<'_> {
        AskDeps {
            chats: &self.store,
            docs: &self.store,
            model: self.ai.model.as_ref(),
            follow_ups: self.ai.follow_ups.as_ref(),
            law: self.law_search(),
            meter: &self.store,
            ids: self.ids.as_ref(),
            clock: self.clock.as_ref(),
            config: AskConfig {
                model_ids: &MODEL_IDS,
                max_tokens: MAX_TOKENS,
                max_history_turns: MAX_HISTORY_TURNS,
                daily_question_cap: DAILY_QUESTION_CAP,
            },
        }
    }

    /// The chat deps.
    #[must_use]
    pub fn chats(&self) -> ChatDeps<'_> {
        ChatDeps {
            chats: &self.store,
            docs: &self.store,
            projects: &self.store,
            clock: self.clock.as_ref(),
            ids: self.ids.as_ref(),
            max_attached_docs: MAX_ATTACHED_DOCS,
        }
    }

    /// The project deps.
    #[must_use]
    pub fn projects(&self) -> ProjectDeps<'_> {
        ProjectDeps {
            projects: &self.store,
            chats: &self.store,
            clock: self.clock.as_ref(),
            ids: self.ids.as_ref(),
        }
    }

    /// The usage meter and clock.
    #[must_use]
    pub fn usage(&self) -> (&SqliteStore, &dyn Clock) {
        (&self.store, self.clock.as_ref())
    }

    /// The client configuration source.
    #[must_use]
    pub fn config(&self) -> ConfigSource<'static> {
        ConfigSource {
            model_ids: &MODEL_IDS,
            labels: &MODEL_LABELS,
            max_attached_docs: MAX_ATTACHED_DOCS,
            daily_question_cap: DAILY_QUESTION_CAP,
        }
    }
}
