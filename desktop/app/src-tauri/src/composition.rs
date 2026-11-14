//! The composition root: builds every adapter once, binds them to the use cases' deps, and holds
//! the running asks. The only place that reads configuration
//! (`kristoff_contracts::config`, `model_config`) and loads the user's Claude key from the
//! Keychain into the shared key slot (Phase 8).

use crate::ai::AiPorts;
use crate::asks::Asks;
use kristoff_adapters::disk_file_store::DiskFileStore;
use kristoff_adapters::keychain::KeychainSecretStore;
use kristoff_adapters::sqlite::SqliteStore;
use kristoff_adapters::system::{RandomIdGenerator, SystemClock};
use kristoff_anthropic::SharedKey;
use kristoff_contracts::config::{DAILY_QUESTION_CAP, MAX_ATTACHED_DOCS, MODEL_IDS, MODEL_LABELS};
use kristoff_contracts::model_config::MAX_TOKENS;
use kristoff_core::domain::parts::SplitLimits;
use kristoff_core::error::AppResult;
use kristoff_core::limits::MAX_HISTORY_TURNS;
use kristoff_core::model::Doc;
use kristoff_core::ports::{
    CaseSearch, Clock, DocStore, FileStore, IdGenerator, LawSearch, SecretStore,
};
use kristoff_core::usecases::ask::{AskConfig, AskDeps};
use kristoff_core::usecases::chats::ChatDeps;
use kristoff_core::usecases::docs::LibraryDeps;
use kristoff_core::usecases::keys::{self, KeyStatus};
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
    /// Where the user's Claude key is kept (the Keychain in production).
    secrets: Box<dyn SecretStore>,
    clock: Box<dyn Clock>,
    ids: Box<dyn IdGenerator>,
    /// The running asks and their cancel flags.
    pub asks: Asks,
    /// The PH law index, installed at start and warmed on a background thread.
    law: OnceLock<Box<dyn LawSearch>>,
    /// The Supreme Court decisions index (Phase 7), installed and warmed like `law`.
    cases: OnceLock<Box<dyn CaseSearch>>,
    /// The background warm-up of `law` and `cases` has finished (successfully or not).
    law_warmed: AtomicBool,
}

impl Composition {
    /// Opens the app data under `data_dir` (`kristoff.db`, `docs/`) with the system clock, the
    /// Anthropic adapters and the Keychain key store.
    ///
    /// # Errors
    /// `INTERNAL` when the database, the docs folder or the Claude client cannot be opened.
    pub fn open(data_dir: &Path) -> AppResult<Self> {
        let ai = AiPorts::anthropic(SharedKey::default())?;
        let secrets = Box::new(KeychainSecretStore::production());
        Self::with_ports(data_dir, Box::new(SystemClock), ai, secrets)
    }

    /// The same with an injected clock, AI ports and key store (tests). Loads the saved key into
    /// the shared slot; a key store that cannot be read is logged and leaves the slot empty (the
    /// user is then asked for a key, never given a guessed one).
    ///
    /// # Errors
    /// `INTERNAL` when the database or the docs folder cannot be opened.
    pub fn with_ports(
        data_dir: &Path,
        clock: Box<dyn Clock>,
        ai: AiPorts,
        secrets: Box<dyn SecretStore>,
    ) -> AppResult<Self> {
        let files = DiskFileStore::open(&data_dir.join("docs"))?;
        match secrets.get_key() {
            Ok(saved) => ai.key.replace(saved),
            Err(e) => ai.log.warn("key.load_failed", e.code.as_str()),
        }
        Ok(Self {
            store: SqliteStore::open(&data_dir.join("kristoff.db"))?,
            files,
            ai,
            secrets,
            clock,
            ids: Box::new(RandomIdGenerator::new(SystemClock)),
            asks: Asks::default(),
            law: OnceLock::new(),
            cases: OnceLock::new(),
            law_warmed: AtomicBool::new(false),
        })
    }

    /// Makes `law` the law index at once (not warmed: a search before the warm-up ends waits for
    /// it); a second install is ignored.
    pub fn install_law(&self, law: Box<dyn LawSearch>) {
        let _ = self.law.set(law);
    }

    /// Makes `cases` the decisions index at once (not warmed); a second install is ignored.
    pub fn install_cases(&self, cases: Box<dyn CaseSearch>) {
        let _ = self.cases.set(cases);
    }

    /// Warms the installed law and case indexes (a no-op for one that is missing), then records
    /// that nothing is left to load, even when a warm-up failed, so the launch screen never waits
    /// forever.
    ///
    /// # Errors
    /// The first warm-up failure (searches of that index will then fail too).
    pub fn warm_law(&self) -> AppResult<()> {
        let law = self.law_search().map_or(Ok(()), LawSearch::warm);
        let cases = self.case_search().map_or(Ok(()), CaseSearch::warm);
        self.law_warmed.store(true, Ordering::SeqCst);
        law.and(cases)
    }

    /// The decisions index, once installed; `None` when the build has no case data.
    #[must_use]
    pub fn case_search(&self) -> Option<&dyn CaseSearch> {
        self.cases.get().map(AsRef::as_ref)
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
            key_configured: self.ai_ready(),
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

    /// The ask deps, with the daily limit in force (the user's, else the default).
    ///
    /// # Errors
    /// `INTERNAL` when the limit cannot be read (fail closed: no question without a known limit).
    pub fn ask(&self) -> AppResult<AskDeps<'_>> {
        let daily_question_cap = self.daily_question_cap()?;
        Ok(AskDeps {
            chats: &self.store,
            docs: &self.store,
            model: self.ai.model.as_ref(),
            follow_ups: self.ai.follow_ups.as_ref(),
            law: self.law_search(),
            cases: self.case_search(),
            meter: &self.store,
            ids: self.ids.as_ref(),
            clock: self.clock.as_ref(),
            config: AskConfig {
                model_ids: &MODEL_IDS,
                max_tokens: MAX_TOKENS,
                max_history_turns: MAX_HISTORY_TURNS,
                daily_question_cap,
            },
            key_configured: self.ai_ready(),
        })
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

    /// The client configuration source, with the daily limit in force.
    ///
    /// # Errors
    /// `INTERNAL` when the limit cannot be read.
    pub fn config(&self) -> AppResult<ConfigSource<'static>> {
        Ok(ConfigSource {
            model_ids: &MODEL_IDS,
            labels: &MODEL_LABELS,
            max_attached_docs: MAX_ATTACHED_DOCS,
            daily_question_cap: self.daily_question_cap()?,
        })
    }

    /// Whether a Claude key is loaded, so asking and uploads can work.
    #[must_use]
    pub fn ai_ready(&self) -> bool {
        self.ai.key.is_set()
    }

    /// The saved key's status (never the key).
    ///
    /// # Errors
    /// `INTERNAL` when the key store cannot be read.
    pub fn key_status(&self) -> AppResult<KeyStatus> {
        keys::key_status(self.secrets.as_ref())
    }

    /// Checks and saves `raw`, then loads the saved key into the shared slot, so the next Claude
    /// call uses it.
    ///
    /// # Errors
    /// `KEY_INVALID`, `UPSTREAM_UNAVAILABLE`, `INTERNAL` (see `keys::save_key`).
    pub fn save_key(&self, raw: &str) -> AppResult<KeyStatus> {
        let status = keys::save_key(self.secrets.as_ref(), self.ai.verifier.as_ref(), raw)?;
        self.ai.key.replace(self.secrets.get_key()?);
        Ok(status)
    }

    /// Deletes the saved key and empties the shared slot.
    ///
    /// # Errors
    /// `INTERNAL` when the key store cannot be written.
    pub fn remove_key(&self) -> AppResult<KeyStatus> {
        let status = keys::remove_key(self.secrets.as_ref())?;
        self.ai.key.replace(None);
        Ok(status)
    }

    /// The daily question limit in force.
    ///
    /// # Errors
    /// `INTERNAL` on a storage failure.
    pub fn daily_question_cap(&self) -> AppResult<u32> {
        keys::daily_question_cap(&self.store, DAILY_QUESTION_CAP)
    }

    /// Saves a new daily question limit.
    ///
    /// # Errors
    /// `VALIDATION_FAILED` outside 1..=10000; `INTERNAL` on a storage failure.
    pub fn set_daily_question_cap(&self, cap: u32) -> AppResult<u32> {
        keys::update_daily_question_cap(&self.store, cap)
    }
}
