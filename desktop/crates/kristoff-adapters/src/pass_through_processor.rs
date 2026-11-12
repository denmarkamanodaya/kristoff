//! The Phase-1 `DocProcessor`: finds nothing, so a confirmed doc becomes `ready` with no page
//! count and no parts. Phase 2 replaces it with the lopdf splitter plus the Files API upload.

use kristoff_core::error::AppResult;
use kristoff_core::model::Doc;
use kristoff_core::ports::{DocProcessor, ProcessedDoc};

/// Processes nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct PassThroughDocProcessor;

impl DocProcessor for PassThroughDocProcessor {
    fn process(&self, _doc: &Doc) -> AppResult<ProcessedDoc> {
        Ok(ProcessedDoc::default())
    }
}
