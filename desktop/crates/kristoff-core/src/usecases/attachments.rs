//! Loads a chat's attachment ids against the docs in one batch read: the shared first step of
//! attaching (all must be usable) and asking (each becomes a model document). Ported from
//! `backend/src/usecases/load-attachments/v1.ts`.

use crate::domain::attachment::{AttachmentRef, parse_attachment_id};
use crate::error::AppResult;
use crate::model::Doc;
use crate::ports::DocStore;

/// One attachment id as loaded: its parsed reference (`None` when malformed) and its doc (`None`
/// when malformed or missing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    /// The parsed id.
    pub reference: Option<AttachmentRef>,
    /// The doc it names.
    pub doc: Option<Doc>,
}

/// One [`Loaded`] per id, in order.
///
/// # Errors
/// Store failures.
pub fn load_attachments(docs: &dyn DocStore, attachment_ids: &[String]) -> AppResult<Vec<Loaded>> {
    let refs: Vec<_> = attachment_ids
        .iter()
        .map(|id| parse_attachment_id(id))
        .collect();
    let named: Vec<String> = refs.iter().flatten().map(|r| r.doc_id.clone()).collect();
    let found = if named.is_empty() {
        vec![]
    } else {
        docs.get_docs(&named)?
    };
    Ok(refs
        .into_iter()
        .map(|reference| {
            let doc = reference
                .as_ref()
                .and_then(|r| found.iter().find(|d| d.id == r.doc_id).cloned());
            Loaded { reference, doc }
        })
        .collect())
}
