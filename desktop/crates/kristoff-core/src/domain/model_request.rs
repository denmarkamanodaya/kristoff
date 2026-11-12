//! What the ask use case sends to the answer model and what comes back (docs/contract.md "Model
//! call"). Ported from `backend/src/domain/chats/model-request.ts`, with Files API ids instead of
//! presigned URLs (spike S4) and a seam for the statute tool loop (Phase 4).

use crate::model::{Role, Usage};

/// One attached document as the model sees it: a whole doc or one part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedDoc {
    /// Library doc id.
    pub doc_id: String,
    /// Library title (citations show this).
    pub library_title: String,
    /// Title sent to the model: the library title, plus `" (pp. a–b)"` for a part.
    pub title: String,
    /// Files API id of the doc or part.
    pub file_id: String,
    /// Pages to add to a part-relative page to get the original page (0 for a whole doc).
    pub page_offset: u32,
}

/// One block of a model turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnBlock {
    /// Plain text.
    Text(String),
    /// A cited document; the cache breakpoint goes on the last one.
    Doc {
        /// Files API id.
        file_id: String,
        /// Title sent to the model.
        title: String,
        /// Whether `cache_control` goes on this block.
        is_cache_breakpoint: bool,
    },
}

/// One turn of the conversation sent to the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTurn {
    /// Who spoke.
    pub role: Role,
    /// Its blocks, in order.
    pub blocks: Vec<TurnBlock>,
}

/// One finished statute tool round (Phase 4): the assistant content exactly as the API produced
/// it, as raw JSON, and the answers sent back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRound {
    /// The assistant content blocks as a JSON array, verbatim (D-11), thinking blocks included.
    pub assistant_content_json: String,
    /// One answer per `tool_use` block, in the same order.
    pub answers: Vec<ToolAnswer>,
}

/// The answer to one `tool_use` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolAnswer {
    /// The `tool_use` id being answered.
    pub tool_use_id: String,
    /// What goes back.
    pub content: ToolAnswerContent,
}

/// What a tool answer carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolAnswerContent {
    /// Search results in rank order (none: a short "nothing found" text is sent).
    Results(Vec<SearchResult>),
    /// A refused call (bad input, unknown tool, over the cap), sent with `is_error: true`.
    Error(String),
}

/// The arguments of one `search_ph_laws` call as the model sent them, unvalidated.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawSearchInput {
    /// Search text.
    pub query: Option<String>,
    /// Law to restrict to, e.g. `RA 9262`.
    pub law_number: Option<String>,
    /// Earliest approval year.
    pub year_from: Option<i64>,
    /// Latest approval year.
    pub year_to: Option<i64>,
    /// Most results.
    pub limit: Option<i64>,
}

/// The arguments of one `get_law_article` call as the model sent them, unvalidated.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawArticleInput {
    /// The law, e.g. `RA 9262` or `Constitution`.
    pub law_number: Option<String>,
    /// The provision, e.g. `Sec. 5`.
    pub article: Option<String>,
}

/// One tool call as the adapter read it; core validates the values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolInput {
    /// `search_ph_laws`.
    SearchLaws(RawSearchInput),
    /// `get_law_article`.
    GetLawArticle(RawArticleInput),
    /// A known tool whose input was not an object of the expected field types.
    Malformed(String),
    /// A tool this app does not offer.
    Unknown(String),
}

/// One `tool_use` block of a finished turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    /// The `tool_use` id.
    pub id: String,
    /// What the model asked for.
    pub input: ToolInput,
}

/// One statute search result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    /// Official source link.
    pub source: String,
    /// Title, for example `"RA 9262 · Sec. 26"`.
    pub title: String,
    /// The article text.
    pub text: String,
}

/// One answer-model call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRequest {
    /// Model id (already allowlisted).
    pub model: String,
    /// `max_tokens`.
    pub max_tokens: u32,
    /// Alternating turns that start and end with a user turn.
    pub turns: Vec<ModelTurn>,
    /// Whether the statute tools are offered.
    pub statutes_on: bool,
    /// Tool rounds already run for this question.
    pub tool_rounds: Vec<ToolRound>,
    /// The tool-call cap is reached: send `tool_choice: none` so the model answers.
    pub tool_choice_none: bool,
}

/// A citation exactly as the model reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitationSource {
    /// A page range of a document block.
    Page {
        /// Index of the document block in the request.
        document_index: usize,
        /// The title the request gave that document.
        document_title: Option<String>,
        /// First page, 1-based, relative to the sent document.
        start_page: u32,
        /// One past the last page.
        end_page_exclusive: u32,
        /// The cited text as sent (may contain `\r\n`).
        cited_text: String,
    },
    /// A `search_result` block returned by the statute tool.
    SearchResult {
        /// The result's source link.
        source: String,
        /// The result's title.
        title: String,
        /// The cited text.
        cited_text: String,
    },
    /// Any other location type, kept for its document.
    Other {
        /// Index of the document block, when the location names one.
        document_index: Option<usize>,
        /// The title the request gave that document.
        document_title: Option<String>,
        /// The cited text.
        cited_text: String,
    },
}

/// One streamed piece of an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelEvent {
    /// Answer text to append.
    Text(String),
    /// A citation for the text that follows.
    Citation(CitationSource),
}

/// How a finished model turn ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnswerEnd {
    /// Token usage of the call.
    pub usage: Usage,
    /// `end_turn`, `max_tokens`, `tool_use`, `refusal`, ...; `None` when the API sent none.
    pub stop_reason: Option<String>,
    /// The `tool_use` blocks of this turn (empty unless `stop_reason` is `tool_use`).
    pub tool_calls: Vec<ToolCall>,
    /// The turn's assistant content blocks as a JSON array, verbatim, for the next tool round.
    pub assistant_content_json: String,
}

impl AnswerEnd {
    /// A turn that ended without tool calls.
    #[must_use]
    pub fn finished(usage: Usage, stop_reason: &str) -> Self {
        Self {
            usage,
            stop_reason: Some(stop_reason.to_owned()),
            ..Self::default()
        }
    }
}

/// Suggested follow-up questions and what the call cost.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FollowUps {
    /// Validated questions (possibly none).
    pub questions: Vec<String>,
    /// Usage, when the call returned a message.
    pub usage: Option<Usage>,
}
