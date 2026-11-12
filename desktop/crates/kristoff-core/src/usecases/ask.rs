//! The ask use case (docs/desktop/phases/phase-2-ask-flow.md §3). Ported from
//! `backend/src/usecases/ask-question/v1.ts:303-336` without the kill switch. Order: model
//! allowlist, chat, attachments → file ids (before the meter: a deliberate deviation), daily meter
//! (one conditional write BEFORE the model call), history; then the user message (and the
//! auto-title), the stream, follow-ups, and ALWAYS the assistant message (with a note when the
//! answer failed or was cut off); then usage.

use crate::domain::citation::map_citation;
use crate::domain::model_request::{AnswerEnd, AttachedDoc, ModelEvent, ModelRequest};
use crate::domain::model_turns::build_model_turns;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::limits::{FOLLOW_UP_ANSWER_CHARS, MAX_CITATIONS, MAX_MESSAGE_TEXT};
use crate::model::{Citation, Message, Role, Usage};
use crate::ports::{
    AnswerModel, AnswerSink, CancelSignal, ChatStore, Clock, DocStore, FollowUpSuggester,
    IdGenerator, UsageMeter,
};
use crate::util::text::truncate_utf16;
use crate::util::time::{seconds_until_next_utc_day, utc_day};

mod attach;
mod store;

pub use store::{CUT_OFF_NOTE, FAILED_NOTE};

/// Model settings the composition root passes in.
#[derive(Debug, Clone, Copy)]
pub struct AskConfig<'a> {
    /// Allowed model ids, first = default.
    pub model_ids: &'a [&'a str],
    /// `max_tokens` of an answer.
    pub max_tokens: u32,
    /// Most past messages sent.
    pub max_history_turns: usize,
    /// Questions per UTC day.
    pub daily_question_cap: u32,
}

/// What the ask use case needs.
pub struct AskDeps<'a> {
    /// Chats and messages.
    pub chats: &'a dyn ChatStore,
    /// Documents.
    pub docs: &'a dyn DocStore,
    /// The answer model.
    pub model: &'a dyn AnswerModel,
    /// The follow-up suggester.
    pub follow_ups: &'a dyn FollowUpSuggester,
    /// The daily meter and token counters.
    pub meter: &'a dyn UsageMeter,
    /// Ids.
    pub ids: &'a dyn IdGenerator,
    /// Time.
    pub clock: &'a dyn Clock,
    /// Model settings.
    pub config: AskConfig<'a>,
}

/// One question: the chat, its text and the model the client chose (`None` = the default).
#[derive(Debug, Clone, Copy)]
pub struct Question<'q> {
    /// The chat id, already format-checked.
    pub chat_id: &'q str,
    /// The question, already length-checked.
    pub text: &'q str,
    /// Requested model id.
    pub model: Option<&'q str>,
}

/// The stored answer and the call's token usage (the `done` event).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskOutcome {
    /// The stored assistant message.
    pub message: Message,
    /// Usage of the answer call.
    pub usage: Usage,
}

/// The answer as it streamed, and how it ended.
struct Answer {
    text: String,
    citations: Vec<Citation>,
    end: AppResult<AnswerEnd>,
    model: String,
}

/// The requested model if allowed, the default when none was asked for; else `VALIDATION_FAILED`.
fn choose_model(allowed: &[&str], requested: Option<&str>) -> AppResult<String> {
    let chosen = match requested {
        None => allowed.first().copied(),
        Some(id) => allowed.iter().copied().find(|a| *a == id),
    };
    chosen
        .map(str::to_owned)
        .ok_or_else(|| AppError::new(ErrorCode::ValidationFailed, "this model is not available"))
}

/// Counts the question for today, before any model call.
fn admit(deps: &AskDeps<'_>, now_ms: i64) -> AppResult<()> {
    let admitted = deps
        .meter
        .admit_question(&utc_day(now_ms), deps.config.daily_question_cap);
    admitted.map_err(|e| match e.code {
        ErrorCode::DailyCapReached => AppError::new(e.code, "daily question limit reached")
            .with_retry_after(seconds_until_next_utc_day(now_ms)),
        _ => e,
    })
}

fn stream_answer(
    deps: &AskDeps<'_>,
    request: &ModelRequest,
    docs: &[AttachedDoc],
    io: (&mut dyn AnswerSink, &dyn CancelSignal),
) -> Answer {
    let (sink, cancel) = io;
    let mut text = String::new();
    let mut citations: Vec<Citation> = Vec::new();
    sink.open();
    let end = deps
        .model
        .stream(request, cancel, &mut |event| match event {
            ModelEvent::Text(piece) => {
                text.push_str(&piece);
                sink.text(&piece);
            }
            ModelEvent::Citation(source) if citations.len() < MAX_CITATIONS => {
                let n = u32::try_from(citations.len() + 1).unwrap_or(u32::MAX);
                let citation = map_citation(&source, docs, n);
                text.push_str(" [");
                text.push_str(&n.to_string());
                text.push(']');
                sink.citation(&citation);
                citations.push(citation);
            }
            ModelEvent::Citation(_) => {}
        });
    let end = if cancel.is_cancelled() && end.is_ok() {
        Err(AppError::new(
            ErrorCode::UpstreamUnavailable,
            "the answer was cancelled",
        ))
    } else {
        end
    };
    Answer {
        text,
        citations,
        end,
        model: request.model.clone(),
    }
}

/// Answers one question in a chat, streaming into `sink`.
///
/// # Errors
/// Before `sink.open()`, in this order: `VALIDATION_FAILED` (model not allowed), `CHAT_NOT_FOUND`,
/// `DOC_NOT_FOUND` (an attachment is missing, not ready or not uploaded; not counted against the
/// cap), `DAILY_CAP_REACHED` (with the seconds to the next UTC day) or a store failure. After it: `UPSTREAM_UNAVAILABLE` (the
/// call failed or was cancelled; both messages are stored, the answer with `FAILED_NOTE` and
/// `stopReason: "error"`) or a store failure.
pub fn ask(
    deps: &AskDeps<'_>,
    question: Question<'_>,
    sink: &mut dyn AnswerSink,
    cancel: &dyn CancelSignal,
) -> AppResult<AskOutcome> {
    let started_ms = deps.clock.now_ms();
    let chat_id = question.chat_id;
    let model = choose_model(deps.config.model_ids, question.model)?;
    let chat = deps.chats.get_chat(chat_id)?;
    // Deliberate deviation from ask-question/v1.ts (which metered first): attachments are resolved
    // BEFORE the daily meter, so a question refused with DOC_NOT_FOUND is not counted.
    let docs = attach::attached_docs(deps.docs, &chat.value.doc_ids)?;
    admit(deps, started_ms)?;
    // At least one message is read, so "no messages yet" is known even when no history is sent.
    let history = deps
        .chats
        .recent_messages(chat_id, deps.config.max_history_turns.max(1))?;
    store::store_question(deps, &chat, question.text, history.is_empty())?;
    let request = ModelRequest {
        model,
        max_tokens: deps.config.max_tokens,
        turns: build_model_turns(
            &history,
            question.text,
            &docs,
            deps.config.max_history_turns,
        ),
        statutes_on: false,
        tool_rounds: vec![],
    };
    let answer = stream_answer(deps, &request, &docs, (sink, cancel));
    // Follow-ups only after a successful answer (a cancelled one already ended as a failure).
    let follow_ups = match &answer.end {
        Ok(_) => {
            let excerpt = truncate_utf16(&answer.text, FOLLOW_UP_ANSWER_CHARS);
            let suggested = deps.follow_ups.suggest(question.text, excerpt);
            sink.follow_ups(&suggested.questions);
            suggested
        }
        Err(_) => crate::domain::model_request::FollowUps::default(),
    };
    store::store_answer(deps, chat_id, (answer, follow_ups), started_ms)
}

/// The stored text: the streamed text plus the note, capped at 200,000 UTF-16 units.
fn final_text(text: &str, note: &str) -> String {
    truncate_utf16(&format!("{text}{note}"), MAX_MESSAGE_TEXT).to_owned()
}

/// A stored message with the fields every message shares.
fn message(id: String, role: Role, text: String, at_ms: i64) -> Message {
    Message {
        id,
        role,
        text,
        citations: vec![],
        usage: None,
        model: None,
        stop_reason: None,
        at_ms,
        duration_ms: None,
        follow_ups: vec![],
    }
}
