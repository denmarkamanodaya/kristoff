//! The PDF viewer (decision D-12): which window shows a doc or part at a page, and the bytes the
//! `kristoff://` scheme serves. Both refuse a doc that is missing, not ready, or lacks the part.

use crate::domain::doc_path::{viewer_label, viewer_url};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::model::{Doc, DocStatus};
use crate::ports::{DocStore, FileStore};

/// Highest page a viewer URL may name.
const MAX_PAGE: u32 = 1_000_000;

/// A viewer window to open (or focus, when one with the label is open).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocWindow {
    /// Unique per doc and part.
    pub label: String,
    /// `kristoff://doc/<id>[/part-<n>][#page=<N>]`.
    pub url: String,
    /// The doc title (the window title).
    pub title: String,
}

/// The doc when it is ready and, if `part` is given, has that part.
///
/// # Errors
/// `DOC_NOT_FOUND` otherwise.
pub fn ready_doc(docs: &dyn DocStore, doc_id: &str, part: Option<u32>) -> AppResult<Doc> {
    let doc = docs.get_doc(doc_id)?.value;
    if doc.status != DocStatus::Ready {
        return Err(AppError::new(ErrorCode::DocNotFound, "not uploaded"));
    }
    if let Some(n) = part
        && !doc.parts.iter().any(|p| p.n == n)
    {
        return Err(AppError::new(ErrorCode::DocNotFound, "no such part"));
    }
    Ok(doc)
}

/// The viewer window of a ready doc or part, at `page` (1-based within that file) when given.
///
/// # Errors
/// `VALIDATION_FAILED` for page 0 or a page past 1 000 000; `DOC_NOT_FOUND` as [`ready_doc`].
pub fn doc_window(
    docs: &dyn DocStore,
    doc_id: &str,
    part: Option<u32>,
    page: Option<u32>,
) -> AppResult<DocWindow> {
    if page.is_some_and(|p| p == 0 || p > MAX_PAGE) {
        return Err(AppError::new(
            ErrorCode::ValidationFailed,
            "page is invalid",
        ));
    }
    let doc = ready_doc(docs, doc_id, part)?;
    let label = viewer_label(doc_id, part);
    let title = match part {
        None => doc.title,
        Some(n) => format!("{} (part {n})", doc.title),
    };
    Ok(DocWindow {
        label,
        url: viewer_url(doc_id, part, page),
        title,
    })
}

/// The PDF bytes of a ready doc or part, for the scheme handler.
///
/// # Errors
/// `DOC_NOT_FOUND` as [`ready_doc`] or when the file is gone; `INTERNAL` on an IO failure.
pub fn read_doc_file(
    docs: &dyn DocStore,
    files: &dyn FileStore,
    doc_id: &str,
    part: Option<u32>,
) -> AppResult<Vec<u8>> {
    ready_doc(docs, doc_id, part)?;
    files.read_stored(doc_id, part)
}
