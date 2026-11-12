//! `LopdfSplitter`: the `PdfSplitter` port on lopdf (docs/desktop/phases/phase-2-ask-flow.md §1).
//! The part rules (100 pages, 30 MiB, halving, 200 parts) are core's
//! (`kristoff_core::domain::parts`, `usecases::processing`); this adapter only counts pages and
//! writes a copy of a PDF that keeps one page range.

use kristoff_core::domain::parts::PageRange;
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::ports::PdfSplitter;
use lopdf::Document;

/// Reads and cuts PDFs with lopdf.
#[derive(Debug, Default, Clone, Copy)]
pub struct LopdfSplitter;

fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::UploadInvalid, message)
}

/// Loads a PDF, refusing an unreadable, encrypted or empty one.
fn load(pdf: &[u8]) -> AppResult<Document> {
    let doc = Document::load_mem(pdf).map_err(|_| invalid("the file is not a readable PDF"))?;
    if doc.is_encrypted() {
        return Err(invalid("the PDF is encrypted"));
    }
    if doc.get_pages().is_empty() {
        return Err(invalid("the PDF has no pages"));
    }
    Ok(doc)
}

fn page_total(doc: &Document) -> u32 {
    u32::try_from(doc.get_pages().len()).unwrap_or(u32::MAX)
}

impl PdfSplitter for LopdfSplitter {
    fn page_count(&self, pdf: &[u8]) -> AppResult<u32> {
        Ok(page_total(&load(pdf)?))
    }

    fn extract(&self, pdf: &[u8], range: PageRange) -> AppResult<Vec<u8>> {
        let mut doc = load(pdf)?;
        let total = page_total(&doc);
        if range.start_page == 0 || range.is_empty() || range.end_page > total {
            return Err(invalid("the page range is outside the PDF"));
        }
        let drop: Vec<u32> = (1..=total)
            .filter(|p| *p < range.start_page || *p > range.end_page)
            .collect();
        doc.delete_pages(&drop);
        doc.prune_objects();
        let mut out = Vec::new();
        doc.save_to(&mut out).map_err(|e| {
            AppError::new(ErrorCode::Internal, format!("could not write a part: {e}"))
        })?;
        Ok(out)
    }
}
