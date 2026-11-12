//! Upload use cases (decision D-8): the web writes the PDF in chunks at explicit offsets into the
//! doc's `.partial`, then confirms. Confirm checks the size and the `%PDF-` magic (ported from
//! `confirm-doc/v1.ts`), commits the bytes and hands the doc to the `DocProcessor` seam; Phase 1
//! binds a no-op processor, so the doc becomes `ready` with no page count and no parts.

use crate::error::{AppError, AppResult, ErrorCode};
use crate::limits::{MAX_UPLOAD_CHUNK_BYTES, PDF_MAGIC};
use crate::model::{Doc, DocStatus};
use crate::ports::{DocPatch, DocProcessor, DocStore, FileStore};

/// What the upload use cases need.
pub struct UploadDeps<'a> {
    /// Documents.
    pub docs: &'a dyn DocStore,
    /// PDF bytes.
    pub files: &'a dyn FileStore,
    /// Page count and split (no-op in Phase 1).
    pub processor: &'a dyn DocProcessor,
}

/// One chunk of an upload.
#[derive(Debug, Clone, Copy)]
pub struct Chunk<'b> {
    /// Where the bytes go in the file.
    pub offset: u64,
    /// The bytes.
    pub bytes: &'b [u8],
}

fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::UploadInvalid, message)
}

/// Writes one chunk of a pending doc's upload; returns the bytes written so far.
///
/// # Errors
/// `DOC_NOT_FOUND`; `UPLOAD_INVALID` when the doc is not pending, the chunk is empty or too big,
/// the offset is past the bytes written so far, or the chunk ends past the declared size.
pub fn upload_chunk(deps: &UploadDeps<'_>, doc_id: &str, chunk: Chunk<'_>) -> AppResult<u64> {
    let doc = deps.docs.get_doc(doc_id)?.value;
    if doc.status != DocStatus::Pending {
        return Err(invalid("the upload is already confirmed"));
    }
    if chunk.bytes.is_empty() || chunk.bytes.len() > MAX_UPLOAD_CHUNK_BYTES {
        return Err(invalid("chunk size is out of range"));
    }
    let written = deps.files.partial_len(doc_id)?.unwrap_or(0);
    if chunk.offset > written {
        return Err(invalid("offset is past the bytes written so far"));
    }
    let len = u64::try_from(chunk.bytes.len()).map_err(|_| invalid("chunk too big"))?;
    if chunk.offset.saturating_add(len) > doc.size {
        return Err(invalid("chunk ends past the declared size"));
    }
    deps.files.write_partial(doc_id, chunk.offset, chunk.bytes)
}

fn check_upload(files: &dyn FileStore, doc: &Doc) -> AppResult<()> {
    let Some(len) = files.partial_len(&doc.id)? else {
        return Err(invalid("no file was uploaded"));
    };
    if len != doc.size {
        return Err(invalid("size does not match"));
    }
    if files.partial_head(&doc.id, PDF_MAGIC.len())? != PDF_MAGIC.as_bytes() {
        return Err(invalid("the file is not a PDF"));
    }
    Ok(())
}

/// Marks the doc `failed` after its processing failed (the web then offers delete), best effort:
/// the processing failure is the error returned whether or not the mark succeeds (as in TS).
fn mark_failed(docs: &dyn DocStore, doc_id: &str, version: i64, failure: AppError) -> AppError {
    let patch = DocPatch {
        status: Some(DocStatus::Failed),
        ..DocPatch::default()
    };
    let _marked = docs.update_doc(doc_id, &patch, version);
    failure
}

/// Confirms a pending doc. Idempotent: a doc past `pending` is returned unchanged.
///
/// # Errors
/// `DOC_NOT_FOUND`, `UPLOAD_INVALID` (missing, size mismatch, not a PDF), `VERSION_CONFLICT`,
/// processor failures (the doc is then `failed`) and store failures.
pub fn confirm_doc(deps: &UploadDeps<'_>, doc_id: &str) -> AppResult<Doc> {
    let found = deps.docs.get_doc(doc_id)?;
    if found.value.status != DocStatus::Pending {
        return Ok(found.value);
    }
    check_upload(deps.files, &found.value)?;
    deps.files.commit_partial(doc_id)?;
    let processed = match deps.processor.process(&found.value) {
        Ok(processed) => processed,
        Err(e) => return Err(mark_failed(deps.docs, doc_id, found.version, e)),
    };
    let patch = DocPatch {
        status: Some(DocStatus::Ready),
        page_count: processed.page_count,
        parts: Some(processed.parts),
        ..DocPatch::default()
    };
    deps.docs.update_doc(doc_id, &patch, found.version)
}
