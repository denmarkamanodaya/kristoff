//! Driven ports: what the core needs from outside. Plain traits, never versioned. The app is
//! single-owner and local, so unlike the web backend no method takes an owner id.
//! Each port has an in-memory fake and a contract suite in `kristoff-testkit`.

use crate::error::AppResult;
use crate::model::{
    Chat, Doc, DocPart, DocStatus, Message, Page, Project, TotpState, Usage, UsageReport, Versioned,
};

/// Reads the current time.
pub trait Clock: Send + Sync {
    /// Epoch milliseconds now.
    fn now_ms(&self) -> i64;
}

impl<T: Clock + ?Sized> Clock for std::sync::Arc<T> {
    fn now_ms(&self) -> i64 {
        (**self).now_ms()
    }
}

/// Makes new ids.
pub trait IdGenerator: Send + Sync {
    /// A new doc, chat or project id: 16 lowercase hex that sort by creation time.
    fn new_id(&self) -> String;
    /// A new message id: a ULID that sorts by time.
    fn new_ulid(&self) -> String;
}

/// Fields a doc update may change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocPatch {
    /// New title.
    pub title: Option<String>,
    /// New status.
    pub status: Option<DocStatus>,
    /// Page count, once known.
    pub page_count: Option<u32>,
    /// Parts, once split.
    pub parts: Option<Vec<DocPart>>,
}

impl DocPatch {
    /// Applies the patch to `doc` (the one rule every `DocStore` uses).
    pub fn apply_to(&self, doc: &mut Doc) {
        if let Some(title) = &self.title {
            doc.title.clone_from(title);
        }
        if let Some(status) = self.status {
            doc.status = status;
        }
        if self.page_count.is_some() {
            doc.page_count = self.page_count;
        }
        if let Some(parts) = &self.parts {
            doc.parts.clone_from(parts);
        }
    }
}

/// Stores library documents.
pub trait DocStore: Send + Sync {
    /// Creates a doc; `INTERNAL` when the id already exists.
    ///
    /// # Errors
    /// `INTERNAL` on a duplicate id or a storage failure.
    fn create_doc(&self, doc: &Doc) -> AppResult<()>;
    /// Reads one doc.
    ///
    /// # Errors
    /// `DOC_NOT_FOUND` when absent.
    fn get_doc(&self, doc_id: &str) -> AppResult<Versioned<Doc>>;
    /// Reads several docs; missing ids are simply absent from the result.
    ///
    /// # Errors
    /// `INTERNAL` on a storage failure.
    fn get_docs(&self, doc_ids: &[String]) -> AppResult<Vec<Doc>>;
    /// Lists docs newest first (id descending), at most `limit` per page.
    ///
    /// # Errors
    /// `VALIDATION_FAILED` for a foreign cursor.
    fn list_docs(&self, cursor: Option<&str>, limit: usize) -> AppResult<Page<Doc>>;
    /// Applies a patch if the stored version still equals `version`.
    ///
    /// # Errors
    /// `DOC_NOT_FOUND` when absent; `VERSION_CONFLICT` when the version moved.
    fn update_doc(&self, doc_id: &str, patch: &DocPatch, version: i64) -> AppResult<Doc>;
    /// Deletes the doc; deleting an absent doc is not an error.
    ///
    /// # Errors
    /// `INTERNAL` on a storage failure.
    fn remove_doc(&self, doc_id: &str) -> AppResult<()>;
}

/// How a chat update changes the chat's project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectChange {
    /// Take the chat out of its project.
    Ungroup,
    /// Move the chat into this project.
    MoveTo(String),
}

impl ProjectChange {
    /// The project the chat ends up in.
    #[must_use]
    pub fn target(&self) -> Option<&str> {
        match self {
            Self::Ungroup => None,
            Self::MoveTo(id) => Some(id),
        }
    }
}

/// The fields of a chat to change; `None` leaves a field alone.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatChanges {
    /// New title.
    pub title: Option<String>,
    /// New attachment ids.
    pub doc_ids: Option<Vec<String>>,
    /// Pin or unpin.
    pub pinned: Option<bool>,
    /// Ungroup or move into a project.
    pub project: Option<ProjectChange>,
}

/// A chat update as stored: the changes plus the new `updated_at_ms`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatPatch {
    /// What changes.
    pub changes: ChatChanges,
    /// The new `updated_at_ms` (callers may keep the old one).
    pub updated_at_ms: i64,
}

impl ChatPatch {
    /// Applies the patch to `chat` (the one rule every `ChatStore` uses).
    pub fn apply_to(&self, chat: &mut Chat) {
        let changes = &self.changes;
        if let Some(title) = &changes.title {
            chat.title.clone_from(title);
        }
        if let Some(doc_ids) = &changes.doc_ids {
            chat.doc_ids.clone_from(doc_ids);
        }
        if let Some(pinned) = changes.pinned {
            chat.pinned = pinned;
        }
        if let Some(change) = &changes.project {
            chat.project_id = change.target().map(str::to_owned);
        }
        chat.updated_at_ms = self.updated_at_ms;
    }
}

/// Stores chats and their messages.
pub trait ChatStore: Send + Sync {
    /// Creates a chat.
    ///
    /// # Errors
    /// `INTERNAL` on a duplicate id or a storage failure.
    fn create_chat(&self, chat: &Chat) -> AppResult<()>;
    /// Reads one chat.
    ///
    /// # Errors
    /// `CHAT_NOT_FOUND` when absent.
    fn get_chat(&self, chat_id: &str) -> AppResult<Versioned<Chat>>;
    /// Lists chats newest `updated_at` first (tie-break id descending).
    ///
    /// # Errors
    /// `VALIDATION_FAILED` for a foreign cursor.
    fn list_chats(&self, cursor: Option<&str>, limit: usize) -> AppResult<Page<Chat>>;
    /// Applies a patch if the stored version still equals `version`.
    ///
    /// # Errors
    /// `CHAT_NOT_FOUND` when absent; `VERSION_CONFLICT` when the version moved.
    fn update_chat(&self, chat_id: &str, patch: &ChatPatch, version: i64) -> AppResult<Chat>;
    /// Appends a message; never overwrites an existing id.
    ///
    /// # Errors
    /// `CHAT_NOT_FOUND` when the chat is absent; `INTERNAL` on a duplicate id.
    fn put_message(&self, chat_id: &str, message: &Message) -> AppResult<()>;
    /// The newest `limit` messages of a chat, oldest first.
    ///
    /// # Errors
    /// `INTERNAL` on a storage failure.
    fn recent_messages(&self, chat_id: &str, limit: usize) -> AppResult<Vec<Message>>;
    /// Deletes every message of a chat; repeatable. Returns how many were deleted.
    ///
    /// # Errors
    /// `INTERNAL` on a storage failure.
    fn delete_messages(&self, chat_id: &str) -> AppResult<usize>;
    /// Deletes the chat; deleting an absent chat is not an error.
    ///
    /// # Errors
    /// `INTERNAL` on a storage failure.
    fn delete_chat(&self, chat_id: &str) -> AppResult<()>;
    /// Sets the chat's project to none only if it is still `project_id` (`updated_at` kept).
    ///
    /// # Errors
    /// `INTERNAL` on a storage failure.
    fn clear_project(&self, chat_id: &str, project_id: &str) -> AppResult<()>;
}

/// Stores projects.
pub trait ProjectStore: Send + Sync {
    /// Creates a project, refusing when `cap` projects exist (one conditional write).
    ///
    /// # Errors
    /// `PROJECT_LIMIT_REACHED` at the cap; `INTERNAL` on a duplicate id.
    fn create_project(&self, project: &Project, cap: usize) -> AppResult<()>;
    /// Reads one project.
    ///
    /// # Errors
    /// `PROJECT_NOT_FOUND` when absent.
    fn get_project(&self, project_id: &str) -> AppResult<Versioned<Project>>;
    /// Every project, in any order.
    ///
    /// # Errors
    /// `INTERNAL` on a storage failure.
    fn list_projects(&self) -> AppResult<Vec<Project>>;
    /// Renames if the stored version still equals `version`.
    ///
    /// # Errors
    /// `PROJECT_NOT_FOUND`; `VERSION_CONFLICT` when the version moved.
    fn rename_project(&self, project_id: &str, change: &ProjectRename) -> AppResult<Project>;
    /// Deletes the project.
    ///
    /// # Errors
    /// `PROJECT_NOT_FOUND` when absent.
    fn delete_project(&self, project_id: &str) -> AppResult<()>;
}

/// A project rename at a read version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRename {
    /// The new name.
    pub name: String,
    /// The rename instant, epoch ms.
    pub updated_at_ms: i64,
    /// The version the rename is based on.
    pub version: i64,
}

/// Meters questions per UTC day and counts tokens.
pub trait UsageMeter: Send + Sync {
    /// Counts one question for `day` in a single conditional write, before the model call.
    ///
    /// # Errors
    /// `DAILY_CAP_REACHED` when `cap` questions were already counted.
    fn admit_question(&self, day: &str, cap: u32) -> AppResult<()>;
    /// Adds a call's tokens to the day and the total.
    ///
    /// # Errors
    /// `INTERNAL` on a storage failure.
    fn add_tokens(&self, day: &str, usage: &Usage) -> AppResult<()>;
    /// Reads the counters (zeros when nothing was counted).
    ///
    /// # Errors
    /// `INTERNAL` on a storage failure.
    fn read_usage(&self, day: &str) -> AppResult<UsageReport>;
}

/// Stores the TOTP replay and lockout state with conditional writes.
pub trait AuthStateStore: Send + Sync {
    /// The stored state; version 0 and the default state when nothing is stored.
    ///
    /// # Errors
    /// `INTERNAL` on a storage failure.
    fn get_auth_state(&self) -> AppResult<Versioned<TotpState>>;
    /// Accepts `step`: sets `last_step`, clears failures and lock, only if the stored
    /// `last_step` is lower.
    ///
    /// # Errors
    /// `OTP_REPLAYED` when it is not (that code or a later one was already used).
    fn accept_step(&self, step: i64) -> AppResult<()>;
    /// Writes the state after a failure if the version is unchanged.
    ///
    /// # Errors
    /// `VERSION_CONFLICT` when the version moved.
    fn record_failure(&self, next: &TotpState, version: i64) -> AppResult<()>;
}

/// Stores PDF bytes: an upload in progress (`.partial`), the confirmed PDF and its parts.
pub trait FileStore: Send + Sync {
    /// Bytes written so far to the upload of `doc_id`, or `None` when nothing was written.
    ///
    /// # Errors
    /// `INTERNAL` on an IO failure.
    fn partial_len(&self, doc_id: &str) -> AppResult<Option<u64>>;
    /// Writes `bytes` at `offset` into the upload; returns its length afterwards.
    ///
    /// # Errors
    /// `INTERNAL` on an IO failure.
    fn write_partial(&self, doc_id: &str, offset: u64, bytes: &[u8]) -> AppResult<u64>;
    /// The first `n` bytes of the upload (fewer when it is shorter).
    ///
    /// # Errors
    /// `INTERNAL` on an IO failure.
    fn partial_head(&self, doc_id: &str, n: usize) -> AppResult<Vec<u8>>;
    /// Turns the upload into the stored PDF.
    ///
    /// # Errors
    /// `INTERNAL` on an IO failure.
    fn commit_partial(&self, doc_id: &str) -> AppResult<()>;
    /// The stored PDF of `doc_id`, or of its part `part`.
    ///
    /// # Errors
    /// `DOC_NOT_FOUND` when that file does not exist; `INTERNAL` on an IO failure.
    fn read_stored(&self, doc_id: &str, part: Option<u32>) -> AppResult<Vec<u8>>;
    /// Deletes the PDF, its parts and any upload; deleting nothing is not an error.
    ///
    /// # Errors
    /// `INTERNAL` on an IO failure.
    fn purge(&self, doc_id: &str) -> AppResult<()>;
}

/// What processing a confirmed PDF found out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessedDoc {
    /// Pages, if counted.
    pub page_count: Option<u32>,
    /// Parts, if split.
    pub parts: Vec<DocPart>,
}

/// Processes a confirmed PDF (page count and split). Phase 1 binds a no-op adapter; Phase 2
/// binds the lopdf splitter and the Files API upload.
pub trait DocProcessor: Send + Sync {
    /// Processes the stored PDF of `doc`.
    ///
    /// # Errors
    /// `UPLOAD_INVALID` for an unreadable PDF; `INTERNAL` on an IO failure.
    fn process(&self, doc: &Doc) -> AppResult<ProcessedDoc>;
}
