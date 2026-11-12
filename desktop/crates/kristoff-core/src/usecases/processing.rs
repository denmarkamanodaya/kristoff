//! Document processing (docs/desktop/phases/phase-2-ask-flow.md §1): a confirmed doc is counted,
//! split into parts when it is too big for one Files API document, each part (or the whole PDF)
//! is uploaded, and the doc becomes `ready` with its file ids. Any failure marks it `failed` and
//! deletes what was already uploaded. Runs off the command thread (the app spawns it).

use crate::domain::parts::{PageRange, SplitLimits, halve_range};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::limits::MAX_PROCESSING_ATTEMPTS;
use crate::model::{Doc, DocPart, DocStatus};
use crate::ports::{DocPatch, DocStore, FileStore, PdfSplitter, RemoteFiles};

/// What processing needs.
pub struct ProcessingDeps<'a> {
    /// Documents.
    pub docs: &'a dyn DocStore,
    /// Local PDF bytes (the parts are stored here too, for the viewer).
    pub files: &'a dyn FileStore,
    /// Page count and cutting.
    pub splitter: &'a dyn PdfSplitter,
    /// The Files API.
    pub remote: &'a dyn RemoteFiles,
    /// Page, byte and part limits ([`SplitLimits::CLAUDE`] in the app).
    pub limits: SplitLimits,
}

/// One cut part before upload.
struct Cut {
    range: PageRange,
    bytes: Vec<u8>,
}

/// Cuts `pdf` into parts that fit, measuring each part's real size and halving a part that is
/// too big. Pure apart from the splitter.
///
/// # Errors
/// `UPLOAD_INVALID` when one page alone is too big or more than `max_parts` parts are needed;
/// splitter failures.
fn cut_parts(deps: &ProcessingDeps<'_>, pdf: &[u8], page_count: u32) -> AppResult<Vec<Cut>> {
    let mut queue = std::collections::VecDeque::from(deps.limits.initial_ranges(page_count));
    let mut parts = Vec::new();
    while let Some(range) = queue.pop_front() {
        if parts.len() + queue.len() + 1 > deps.limits.max_parts {
            return Err(AppError::new(
                ErrorCode::UploadInvalid,
                "the PDF needs too many parts",
            ));
        }
        let bytes = deps.splitter.extract(pdf, range)?;
        if deps.limits.fits(bytes.len() as u64) {
            parts.push(Cut { range, bytes });
            continue;
        }
        let Some((first, second)) = halve_range(range) else {
            let message = format!("page {} alone is larger than the limit", range.start_page);
            return Err(AppError::new(ErrorCode::UploadInvalid, message));
        };
        queue.push_front(second);
        queue.push_front(first);
    }
    Ok(parts)
}

/// Stores and uploads each part; `uploaded` collects every file id so a failure can delete them.
fn upload_parts(
    deps: &ProcessingDeps<'_>,
    doc_id: &str,
    cuts: Vec<Cut>,
    uploaded: &mut Vec<String>,
) -> AppResult<Vec<DocPart>> {
    let mut parts = Vec::with_capacity(cuts.len());
    for (n, cut) in (1u32..).zip(cuts) {
        deps.files.write_part(doc_id, n, &cut.bytes)?;
        let file_id = deps
            .remote
            .upload(&format!("{doc_id}-part-{n}.pdf"), &cut.bytes)?;
        uploaded.push(file_id.clone());
        parts.push(DocPart {
            n,
            start_page: cut.range.start_page,
            end_page: cut.range.end_page,
            size: cut.bytes.len() as u64,
            file_id: Some(file_id),
        });
    }
    Ok(parts)
}

/// Splits and uploads; returns the patch that makes the doc ready.
fn prepare(
    deps: &ProcessingDeps<'_>,
    doc_id: &str,
    uploaded: &mut Vec<String>,
) -> AppResult<DocPatch> {
    let pdf = deps.files.read_stored(doc_id, None)?;
    let page_count = deps.splitter.page_count(&pdf)?;
    let ready = DocPatch {
        status: Some(DocStatus::Ready),
        page_count: Some(page_count),
        ..DocPatch::default()
    };
    if !deps.limits.needs_parts(page_count, pdf.len() as u64) {
        let file_id = deps.remote.upload(&format!("{doc_id}.pdf"), &pdf)?;
        uploaded.push(file_id.clone());
        return Ok(DocPatch {
            file_id: Some(file_id),
            parts: Some(vec![]),
            ..ready
        });
    }
    let cuts = cut_parts(deps, &pdf, page_count)?;
    let parts = upload_parts(deps, doc_id, cuts, uploaded)?;
    Ok(DocPatch {
        parts: Some(parts),
        ..ready
    })
}

/// Writes `patch` at the doc's current version (a rename may have moved it meanwhile).
fn patch_now(docs: &dyn DocStore, doc_id: &str, patch: &DocPatch) -> AppResult<Doc> {
    let current = docs.get_doc(doc_id)?;
    docs.update_doc(doc_id, patch, current.version)
}

/// Processes one doc in `processing`: count, split, upload, then `ready`. A doc in any other
/// state is returned unchanged.
///
/// # Errors
/// The failure that stopped it. The doc is then `failed` (best effort: a doc deleted meanwhile
/// stays deleted) and every file already uploaded is deleted.
pub fn process_doc(deps: &ProcessingDeps<'_>, doc_id: &str) -> AppResult<Doc> {
    let found = deps.docs.get_doc(doc_id)?;
    if found.value.status != DocStatus::Processing {
        return Ok(found.value);
    }
    // Counted before any work, so a run that crashes the app still uses up its attempt.
    let attempt = DocPatch {
        processing_attempts: Some(found.value.processing_attempts + 1),
        ..DocPatch::default()
    };
    deps.docs.update_doc(doc_id, &attempt, found.version)?;
    let mut uploaded = Vec::new();
    let outcome =
        prepare(deps, doc_id, &mut uploaded).and_then(|p| patch_now(deps.docs, doc_id, &p));
    outcome.inspect_err(|_| {
        deps.remote.delete(&uploaded);
        let failed = DocPatch {
            status: Some(DocStatus::Failed),
            ..DocPatch::default()
        };
        // Recovery: the processing failure is what the caller reports; if marking `failed` also
        // fails (the doc was deleted meanwhile), there is nothing left to mark.
        let _marked = patch_now(deps.docs, doc_id, &failed);
    })
}

/// Docs to process when the app starts: any left in `processing` with fewer than
/// [`MAX_PROCESSING_ATTEMPTS`] runs (one left after its attempts is marked `failed` instead, so a
/// doc that crashes the app cannot loop), and any `ready` doc without file ids (made by a Phase-1
/// build), which is moved back to `processing` with its attempts reset. Returns their ids.
///
/// # Errors
/// Store failures.
pub fn docs_to_resume(docs: &dyn DocStore) -> AppResult<Vec<String>> {
    let mut ids = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let page = docs.list_docs(cursor.as_deref(), crate::limits::DOCS_PAGE_SIZE)?;
        for doc in &page.items {
            if needs_resume(docs, doc)? {
                ids.push(doc.id.clone());
            }
        }
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => return Ok(ids),
        }
    }
}

fn needs_resume(docs: &dyn DocStore, doc: &Doc) -> AppResult<bool> {
    let set = |status: DocStatus, attempts: Option<u32>| DocPatch {
        status: Some(status),
        processing_attempts: attempts,
        ..DocPatch::default()
    };
    match doc.status {
        DocStatus::Processing if doc.processing_attempts < MAX_PROCESSING_ATTEMPTS => Ok(true),
        DocStatus::Processing => {
            patch_now(docs, &doc.id, &set(DocStatus::Failed, None)).map(|_| false)
        }
        DocStatus::Ready if doc.remote_file_ids().is_empty() => {
            patch_now(docs, &doc.id, &set(DocStatus::Processing, Some(0))).map(|_| true)
        }
        _ => Ok(false),
    }
}
