//! A doc processor that returns a fixed result (or a fixed failure).

use kristoff_core::error::AppResult;
use kristoff_core::model::Doc;
use kristoff_core::ports::{DocProcessor, ProcessedDoc};
use std::sync::{Mutex, PoisonError};

/// Returns `result` for every doc and records the ids it saw.
#[derive(Debug)]
pub struct FakeProcessor {
    result: AppResult<ProcessedDoc>,
    seen: Mutex<Vec<String>>,
}

impl FakeProcessor {
    /// A processor returning `result`.
    #[must_use]
    pub fn returning(result: AppResult<ProcessedDoc>) -> Self {
        Self {
            result,
            seen: Mutex::new(vec![]),
        }
    }

    /// The doc ids processed so far.
    #[must_use]
    pub fn seen(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl DocProcessor for FakeProcessor {
    fn process(&self, doc: &Doc) -> AppResult<ProcessedDoc> {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(doc.id.clone());
        self.result.clone()
    }
}
