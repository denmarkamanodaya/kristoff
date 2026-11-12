//! Response shapes, field for field `backend/src/contracts/shapes.ts` (plus the additive
//! `Citation.kind` and `Citation.sourceUrl`). Optional values serialise as `null`, never omitted.
//! Conversions from domain values: `wire.rs`.

use serde::{Deserialize, Serialize};

/// Token counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UsageDto {
    /// Input tokens.
    pub input: u64,
    /// Output tokens.
    pub output: u64,
    /// Cache-read tokens.
    pub cache_read: u64,
    /// Cache-write tokens.
    pub cache_write: u64,
}

/// One part of a large document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocPartDto {
    /// Part number.
    pub n: u32,
    /// First original page.
    pub start_page: u32,
    /// Last original page.
    pub end_page: u32,
    /// Bytes.
    pub size: u64,
}

/// Upload state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocStatusDto {
    /// Not confirmed.
    Pending,
    /// Being processed.
    Processing,
    /// Usable.
    Ready,
    /// Failed.
    Failed,
}

/// A library document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocDto {
    /// 16-hex id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Bytes.
    pub size: u64,
    /// Upload state.
    pub status: DocStatusDto,
    /// ISO instant.
    pub created_at: String,
    /// Pages, once known.
    pub page_count: Option<u32>,
    /// Parts.
    pub parts: Vec<DocPartDto>,
}

/// What a citation points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CitationKindDto {
    /// An attached document.
    Doc,
    /// A statute.
    Law,
}

/// A cited passage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CitationDto {
    /// Marker number.
    pub n: u32,
    /// Cited doc id, if a doc.
    pub doc_id: Option<String>,
    /// Doc title or statute label.
    pub doc_title: String,
    /// First page.
    pub page: Option<u32>,
    /// Last page.
    pub end_page: Option<u32>,
    /// Cited text.
    pub quote: String,
    /// Doc or law.
    pub kind: CitationKindDto,
    /// Official source of a statute.
    pub source_url: Option<String>,
}

/// A chat as listed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChatSummaryDto {
    /// 16-hex id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Attachment ids.
    pub doc_ids: Vec<String>,
    /// ISO instant.
    pub updated_at: String,
    /// Pinned.
    pub pinned: bool,
    /// Project, or null.
    pub project_id: Option<String>,
}

/// A chat with its creation instant (`ChatWire = ChatSummaryWire.extend({createdAt})`). Flattening
/// rules out `deny_unknown_fields` here; the fixture round trip still fails on any extra key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatDto {
    /// The listed fields.
    #[serde(flatten)]
    pub summary: ChatSummaryDto,
    /// ISO instant.
    pub created_at: String,
}

/// Who wrote a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoleDto {
    /// The student.
    User,
    /// The model.
    Assistant,
}

/// A stored message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessageDto {
    /// ULID.
    pub id: String,
    /// Author.
    pub role: RoleDto,
    /// Text.
    pub text: String,
    /// Citations.
    pub citations: Vec<CitationDto>,
    /// Tokens.
    pub usage: Option<UsageDto>,
    /// Model id.
    pub model: Option<String>,
    /// Stop reason.
    pub stop_reason: Option<String>,
    /// ISO instant.
    pub at: String,
    /// Answer duration.
    pub duration_ms: Option<u64>,
    /// Suggested next questions.
    pub follow_ups: Vec<String>,
}

/// A project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectDto {
    /// 16-hex id.
    pub id: String,
    /// Name.
    pub name: String,
    /// ISO instant.
    pub created_at: String,
    /// ISO instant.
    pub updated_at: String,
}

/// One selectable model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelOptionDto {
    /// API id.
    pub id: String,
    /// Label.
    pub label: String,
}

/// The upload headers the web sends (always the PDF type).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadHeadersDto {
    /// `application/pdf`.
    #[serde(rename = "Content-Type")]
    pub content_type: String,
}
