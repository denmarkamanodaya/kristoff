//! Ports of document processing: the PDF splitter, the remote file store (the Files API) and the
//! warning log for best-effort steps (docs/desktop/phases/phase-2-ask-flow.md §1–§2).

use crate::domain::parts::PageRange;
use crate::error::AppResult;

/// Reads and cuts PDFs.
pub trait PdfSplitter: Send + Sync {
    /// Counts the pages of a PDF.
    ///
    /// # Errors
    /// `UPLOAD_INVALID` for bytes that are not a readable, unencrypted PDF with at least one page.
    fn page_count(&self, pdf: &[u8]) -> AppResult<u32>;
    /// A new PDF holding only `range` of `pdf`.
    ///
    /// # Errors
    /// `UPLOAD_INVALID` for an unreadable PDF or a range outside it; `INTERNAL` when writing fails.
    fn extract(&self, pdf: &[u8], range: PageRange) -> AppResult<Vec<u8>>;
}

/// Stores PDFs where the model reads them (the Files API).
pub trait RemoteFiles: Send + Sync {
    /// Uploads a PDF and returns its file id.
    ///
    /// # Errors
    /// `UPSTREAM_UNAVAILABLE` when the upload fails.
    fn upload(&self, file_name: &str, pdf: &[u8]) -> AppResult<String>;
    /// Deletes files, best effort: never fails; the adapter logs each failure.
    fn delete(&self, file_ids: &[String]);
}

/// Records warnings about best-effort steps. Never receives prompts, answers, titles or keys.
pub trait Logger: Send + Sync {
    /// One warning line: a dotted event name and a short reason (an error code or outcome).
    fn warn(&self, event: &str, reason: &str);
}
