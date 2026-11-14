//! Response envelopes, one per command result (`backend/src/contracts/shapes.ts` names).

use crate::shapes::{
    ChatDto, ChatSummaryDto, DocDto, MessageDto, ModelOptionDto, ProjectDto, UploadHeadersDto,
    UsageDto,
};
use serde::{Deserialize, Serialize};

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
    /// Desktop features (Phase 4).
    pub features: FeaturesDto,
}

/// `get_config` features: statute search and the size of its index.
// Reason: this is a flat wire shape the web already reads (`features.aiReady` beside `statutes`,
// `lawReady`, `cases`; docs/contract.md); grouping the flags into an enum would change the contract.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FeaturesDto {
    /// Statute search is available (the law index is installed).
    pub statutes: bool,
    /// Laws in the index; omitted when the index is unavailable or does not say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub law_count: Option<u64>,
    /// Both indexes have warmed (vectors and model loaded), or there are none: the launch screen
    /// can give way to Home. The `law-ready` event announces the same moment.
    pub law_ready: bool,
    /// Case search is available (the decisions index is installed; Phase 7).
    pub cases: bool,
    /// Decisions in the index; omitted when the index is unavailable or cannot count them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_count: Option<u64>,
    /// A Claude API key is saved, so asking can work (Phase 8). False: the web gates Send and
    /// opens Settings.
    pub ai_ready: bool,
}

/// `get_key_status`, `set_api_key` and `remove_api_key` (Phase 8). Never carries the key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeyStatusResponse {
    /// A key is saved.
    pub configured: bool,
    /// The key's last 4 characters, or null when none is saved.
    pub hint: Option<String>,
}

impl From<&kristoff_core::usecases::keys::KeyStatus> for KeyStatusResponse {
    fn from(s: &kristoff_core::usecases::keys::KeyStatus) -> Self {
        Self {
            configured: s.configured,
            hint: s.hint.clone(),
        }
    }
}

/// `get_settings` and `update_settings` (Phase 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsResponse {
    /// Questions per UTC day, 1..=10000.
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
