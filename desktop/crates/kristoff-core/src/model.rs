//! Domain value types (docs/contract.md "Shapes"). Ports and use cases speak only these; the wire
//! DTOs in `kristoff-contracts` mirror them. Instants are epoch milliseconds (UTC).

/// A stored value with its optimistic-lock version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Versioned<T> {
    /// The value as read.
    pub value: T,
    /// The version a conditional write must match.
    pub version: i64,
}

/// One page of a bounded list. `next_cursor` is opaque; `None` when there is no next page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    /// The items, in list order.
    pub items: Vec<T>,
    /// Echo this to read the next page.
    pub next_cursor: Option<String>,
}

/// Upload state of a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocStatus {
    /// Created; the bytes are not confirmed yet.
    Pending,
    /// Confirmed; being processed.
    Processing,
    /// Usable.
    Ready,
    /// Processing failed.
    Failed,
}

impl DocStatus {
    /// The wire and storage name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }

    /// Parses a wire or storage name.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Pending, Self::Processing, Self::Ready, Self::Failed]
            .into_iter()
            .find(|s| s.as_str() == text)
    }
}

/// One part of a large document: 1-based inclusive original page range and size in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocPart {
    /// Part number, 1-based.
    pub n: u32,
    /// First original page (1-based).
    pub start_page: u32,
    /// Last original page (inclusive).
    pub end_page: u32,
    /// Size in bytes.
    pub size: u64,
    /// Files API id of this part, once uploaded (never on the wire).
    pub file_id: Option<String>,
}

/// A library document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doc {
    /// 16-hex id.
    pub id: String,
    /// Shown title.
    pub title: String,
    /// Declared size in bytes.
    pub size: u64,
    /// Upload state.
    pub status: DocStatus,
    /// Creation instant, epoch ms.
    pub created_at_ms: i64,
    /// Pages, once known.
    pub page_count: Option<u32>,
    /// Parts of a large PDF (empty when attached whole).
    pub parts: Vec<DocPart>,
    /// Files API id of the whole PDF, once uploaded (unsplit docs only; never on the wire).
    pub file_id: Option<String>,
    /// Processing runs started so far (bounds the retries at app start; never on the wire).
    pub processing_attempts: u32,
}

impl Doc {
    /// Every Files API id this doc holds: the whole file and each part's.
    #[must_use]
    pub fn remote_file_ids(&self) -> Vec<String> {
        let parts = self.parts.iter().filter_map(|p| p.file_id.clone());
        self.file_id.iter().cloned().chain(parts).collect()
    }
}

/// A chat. Listed chats carry the same fields; the list shape drops `created_at_ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chat {
    /// 16-hex id.
    pub id: String,
    /// Shown title.
    pub title: String,
    /// Attachment ids (`<docId>` or `<docId>:p<n>`), in request order.
    pub doc_ids: Vec<String>,
    /// Shown in the Pinned group.
    pub pinned: bool,
    /// The project the chat is grouped under.
    pub project_id: Option<String>,
    /// Creation instant, epoch ms.
    pub created_at_ms: i64,
    /// Last change, epoch ms (the list order).
    pub updated_at_ms: i64,
    /// Whether statute search (the law tools) is offered for this chat's questions.
    pub statutes: bool,
}

/// Who wrote a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The student.
    User,
    /// The model.
    Assistant,
}

impl Role {
    /// The wire and storage name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }

    /// Parses a wire or storage name.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::User, Self::Assistant]
            .into_iter()
            .find(|r| r.as_str() == text)
    }
}

/// What a citation points at: an attached document page, or a statute article.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CitationKind {
    /// A passage of an attached document.
    Doc,
    /// A statute article from the law index.
    Law,
    /// A Supreme Court decision from the case index (Phase 7).
    Case,
}

impl CitationKind {
    /// The wire and storage name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Doc => "doc",
            Self::Law => "law",
            Self::Case => "case",
        }
    }

    /// Parses a wire or storage name.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Doc, Self::Law, Self::Case]
            .into_iter()
            .find(|k| k.as_str() == text)
    }
}

/// A cited passage. Pages are 1-based and inclusive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    /// Marker number in the answer text, 1-based.
    pub n: u32,
    /// The cited document, if it is one.
    pub doc_id: Option<String>,
    /// Document title or statute label.
    pub doc_title: String,
    /// First cited page.
    pub page: Option<u32>,
    /// Last cited page.
    pub end_page: Option<u32>,
    /// The cited text.
    pub quote: String,
    /// Document or statute.
    pub kind: CitationKind,
    /// The official source link of a statute; `None` for documents.
    pub source_url: Option<String>,
}

/// Token counts of one model call or a running total.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Usage {
    /// Input tokens.
    pub input: u64,
    /// Output tokens.
    pub output: u64,
    /// Cache-read tokens.
    pub cache_read: u64,
    /// Cache-write tokens.
    pub cache_write: u64,
}

/// One stored chat message. `id` is a ULID, so ids sort by time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// ULID.
    pub id: String,
    /// Author.
    pub role: Role,
    /// Text, with ` [n]` citation markers for answers.
    pub text: String,
    /// Citations of an answer.
    pub citations: Vec<Citation>,
    /// Tokens of an answer.
    pub usage: Option<Usage>,
    /// Model id of an answer.
    pub model: Option<String>,
    /// Stop reason of an answer.
    pub stop_reason: Option<String>,
    /// Write instant, epoch ms.
    pub at_ms: i64,
    /// Answer duration in ms.
    pub duration_ms: Option<u64>,
    /// Suggested next questions (answers only).
    pub follow_ups: Vec<String>,
}

/// A named group of chats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// 16-hex id.
    pub id: String,
    /// Name, 1..80.
    pub name: String,
    /// Creation instant, epoch ms.
    pub created_at_ms: i64,
    /// Last rename, epoch ms.
    pub updated_at_ms: i64,
}

/// Today's (UTC) and all-time token counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UsageReport {
    /// Today's counters.
    pub today: Usage,
    /// All-time counters.
    pub total: Usage,
}
