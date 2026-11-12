//! Response envelopes, one per command result (`backend/src/contracts/shapes.ts` names).

use crate::shapes::{
    ChatDto, ChatSummaryDto, DocDto, MessageDto, ModelOptionDto, ProjectDto, UploadHeadersDto,
    UsageDto,
};
use serde::{Deserialize, Serialize};

/// `verify_otp`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionResponse {
    /// Cosmetic token (`"local"`).
    pub token: String,
    /// When the unlock ends (ISO instant).
    pub expires_at: String,
}

/// `get_config`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigResponse {
    /// Default model id.
    pub model: String,
    /// Allowed models.
    pub models: Vec<ModelOptionDto>,
    /// Attach cap.
    pub max_attached_docs: usize,
    /// Questions per UTC day.
    pub daily_question_cap: u32,
}

/// `list_docs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocsPageResponse {
    /// Docs, newest first.
    pub items: Vec<DocDto>,
    /// Next page, or null.
    pub next_cursor: Option<String>,
}

/// `create_doc`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocUploadResponse {
    /// The pending doc.
    pub doc: DocDto,
    /// `kristoff-upload://<docId>`.
    pub upload_url: String,
    /// Headers the web sends.
    pub upload_headers: UploadHeadersDto,
}

/// `confirm_doc` and `rename_doc`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocResponse {
    /// The doc.
    pub doc: DocDto,
}

/// `get_doc_link`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LinkResponse {
    /// `kristoff://doc/<id>[/part-<n>]`.
    pub url: String,
    /// ISO instant.
    pub expires_at: String,
}

/// `list_chats`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChatsPageResponse {
    /// Chats, newest update first.
    pub items: Vec<ChatSummaryDto>,
    /// Next page, or null.
    pub next_cursor: Option<String>,
}

/// `create_chat` and `update_chat`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChatResponse {
    /// The chat.
    pub chat: ChatDto,
}

/// `get_chat`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChatWithMessagesResponse {
    /// The chat.
    pub chat: ChatDto,
    /// Newest 200, oldest first.
    pub messages: Vec<MessageDto>,
}

/// `get_usage`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UsageResponse {
    /// Today (UTC).
    pub today: UsageDto,
    /// All time.
    pub total: UsageDto,
}

/// `delete_doc`, `delete_chat` and `delete_project`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeletedResponse {
    /// Always true.
    pub deleted: bool,
}

/// `list_projects`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectsResponse {
    /// Projects in name order.
    pub items: Vec<ProjectDto>,
}

/// `create_project` and `rename_project`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectResponse {
    /// The project.
    pub project: ProjectDto,
}

/// `upload_chunk`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UploadChunkResponse {
    /// Bytes written so far.
    pub written: u64,
}
