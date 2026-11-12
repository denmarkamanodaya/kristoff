//! Resolves a chat's attachment ids to the documents sent to the model, in the chat's order.

use crate::domain::attachment::{AttachmentTarget, attachment_target};
use crate::domain::model_request::AttachedDoc;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::ports::DocStore;
use crate::usecases::attachments::load_attachments;

fn not_usable() -> AppError {
    AppError::new(
        ErrorCode::DocNotFound,
        "an attached document is missing or still processing",
    )
}

/// One attached document per id. Unlike the web (which skipped them), an id that is malformed,
/// names a missing or not-ready doc or part, or has no Files API id refuses the question, so the
/// student never gets an answer that silently ignored a document.
///
/// # Errors
/// `DOC_NOT_FOUND` for such an id; store failures.
pub(super) fn attached_docs(
    docs: &dyn DocStore,
    attachment_ids: &[String],
) -> AppResult<Vec<AttachedDoc>> {
    load_attachments(docs, attachment_ids)?
        .iter()
        .map(|l| {
            let r = l.reference.as_ref().ok_or_else(not_usable)?;
            to_attached(&attachment_target(r, l.doc.as_ref()).ok_or_else(not_usable)?)
        })
        .collect()
}

fn to_attached(target: &AttachmentTarget<'_>) -> AppResult<AttachedDoc> {
    match target {
        AttachmentTarget::Whole(doc) => Ok(AttachedDoc {
            doc_id: doc.id.clone(),
            library_title: doc.title.clone(),
            title: doc.title.clone(),
            file_id: doc.file_id.clone().ok_or_else(not_usable)?,
            page_offset: 0,
        }),
        AttachmentTarget::Part(doc, part) => Ok(AttachedDoc {
            doc_id: doc.id.clone(),
            library_title: doc.title.clone(),
            title: format!(
                "{} (pp. {}\u{2013}{})",
                doc.title, part.start_page, part.end_page
            ),
            file_id: part.file_id.clone().ok_or_else(not_usable)?,
            page_offset: part.start_page - 1,
        }),
    }
}
