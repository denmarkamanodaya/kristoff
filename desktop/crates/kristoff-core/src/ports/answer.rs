//! Ports of the ask flow: the answer model, the follow-up suggester, cancellation and the sink the
//! use case streams into (docs/desktop/phases/phase-2-ask-flow.md §2–§3).

use crate::domain::model_request::{AnswerEnd, FollowUps, ModelEvent, ModelRequest};
use crate::error::AppResult;
use crate::model::Citation;

/// Which index a search ran against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    /// The PH law index (`search_ph_laws`).
    Law,
    /// The Supreme Court decisions index (`search_ph_cases`).
    Case,
}

impl SearchKind {
    /// The wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Law => "law",
            Self::Case => "case",
        }
    }
}

/// Tells a running answer to stop (the app's `cancel_ask` and window close set it).
pub trait CancelSignal: Send + Sync {
    /// Whether the answer should stop now.
    fn is_cancelled(&self) -> bool;
}

/// A signal that never fires.
#[derive(Debug, Clone, Copy, Default)]
pub struct NeverCancelled;

impl CancelSignal for NeverCancelled {
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Streams one answer.
pub trait AnswerModel: Send + Sync {
    /// Sends `request` and calls `on_event` for every text piece and citation, in order. Stops
    /// soon after `cancel` fires; events already delivered stay delivered.
    ///
    /// # Errors
    /// `UPSTREAM_UNAVAILABLE` for any failure of the call, the stream or a cancel.
    fn stream(
        &self,
        request: &ModelRequest,
        cancel: &dyn CancelSignal,
        on_event: &mut dyn FnMut(ModelEvent),
    ) -> AppResult<AnswerEnd>;
}

/// Suggests the student's likely next questions.
pub trait FollowUpSuggester: Send + Sync {
    /// Suggestions for `question` given the start of the answer. Never fails: any failure gives
    /// no questions (and the adapter logs it).
    fn suggest(&self, question: &str, answer_excerpt: &str) -> FollowUps;
}

/// Where the ask use case pushes a streamed answer (the `ask` command sends each call as an
/// `AskEvent` on its channel). Ported from `backend/src/ports/answer-sink.ts`.
pub trait AnswerSink {
    /// Called once, after every pre-stream check passed and before the first event. Failures
    /// before it are command errors; after it they are `error` events.
    fn open(&mut self);
    /// A piece of answer text.
    fn text(&mut self, text: &str);
    /// A numbered citation (the stored text has ` [n]` after the text it follows).
    fn citation(&mut self, citation: &Citation);
    /// The suggested follow-up questions, sent once after the answer.
    fn follow_ups(&mut self, questions: &[String]);
    /// A statute or case search ran: its query, how many results it returned and which index
    /// (display only).
    fn search(&mut self, query: &str, count: usize, kind: SearchKind);
}
