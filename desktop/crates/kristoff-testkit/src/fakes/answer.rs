//! Fakes of the ask-flow ports: a scripted answer model, fixed follow-ups and a cancel flag.

use kristoff_core::domain::model_request::{AnswerEnd, FollowUps, ModelEvent, ModelRequest};
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::ports::{AnswerModel, CancelSignal, FollowUpSuggester};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

/// What a scripted model streams, then how it ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    /// Events, in order.
    pub events: Vec<ModelEvent>,
    /// The result after the last event.
    pub end: AppResult<AnswerEnd>,
}

/// Streams a [`Script`] per turn, checking the cancel signal before every event, and records
/// requests. Turn `i` plays script `i` (the last one repeats).
#[derive(Debug)]
pub struct ScriptedModel {
    scripts: Vec<Script>,
    requests: Mutex<Vec<ModelRequest>>,
}

impl ScriptedModel {
    /// A model that plays `script` for every request.
    #[must_use]
    pub fn new(script: Script) -> Self {
        Self::turns(vec![script])
    }

    /// A model that plays `scripts[i]` on its `i`-th request (the last repeats).
    #[must_use]
    pub fn turns(scripts: Vec<Script>) -> Self {
        Self {
            scripts,
            requests: Mutex::new(vec![]),
        }
    }

    /// The requests received so far.
    #[must_use]
    pub fn requests(&self) -> Vec<ModelRequest> {
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// The error a cancelled stream returns.
fn cancelled() -> AppError {
    AppError::new(ErrorCode::UpstreamUnavailable, "the answer was cancelled")
}

impl AnswerModel for ScriptedModel {
    fn stream(
        &self,
        request: &ModelRequest,
        cancel: &dyn CancelSignal,
        on_event: &mut dyn FnMut(ModelEvent),
    ) -> AppResult<AnswerEnd> {
        let turn = {
            let mut requests = self.requests.lock().unwrap_or_else(PoisonError::into_inner);
            requests.push(request.clone());
            requests.len() - 1
        };
        let Some(script) = self.scripts.get(turn).or(self.scripts.last()) else {
            return Err(cancelled());
        };
        for event in &script.events {
            if cancel.is_cancelled() {
                return Err(cancelled());
            }
            on_event(event.clone());
        }
        if cancel.is_cancelled() {
            return Err(cancelled());
        }
        script.end.clone()
    }
}

/// Returns fixed follow-ups and records each `(question, excerpt)`.
#[derive(Debug, Default)]
pub struct FixedFollowUps {
    answer: FollowUps,
    asked: Mutex<Vec<(String, String)>>,
}

impl FixedFollowUps {
    /// A suggester that always returns `answer`.
    #[must_use]
    pub fn returning(answer: FollowUps) -> Self {
        Self {
            answer,
            asked: Mutex::new(vec![]),
        }
    }

    /// Every `(question, answer excerpt)` asked so far.
    #[must_use]
    pub fn asked(&self) -> Vec<(String, String)> {
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl FollowUpSuggester for FixedFollowUps {
    fn suggest(&self, question: &str, answer_excerpt: &str) -> FollowUps {
        let mut asked = self.asked.lock().unwrap_or_else(PoisonError::into_inner);
        asked.push((question.to_owned(), answer_excerpt.to_owned()));
        self.answer.clone()
    }
}

/// A cancel flag a test (or a sink) can set.
#[derive(Debug, Default)]
pub struct CancelFlag(AtomicBool);

impl CancelFlag {
    /// Sets the flag.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl CancelSignal for CancelFlag {
    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
