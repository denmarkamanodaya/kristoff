//! `PdfSplitter` and `RemoteFiles` contracts.

use super::assert_code;
use kristoff_core::domain::parts::PageRange;
use kristoff_core::error::ErrorCode;
use kristoff_core::ports::{PdfSplitter, RemoteFiles};

/// Runs every `PdfSplitter` promise. `make_pdf(n)` returns a PDF with `n` pages that this
/// splitter reads.
pub fn pdf_splitter_contract(splitter: &impl PdfSplitter, make_pdf: impl Fn(u32) -> Vec<u8>) {
    let pdf = make_pdf(7);
    counts_pages_and_extracts_ranges(splitter, &pdf);
    refuses_ranges_outside_and_non_pdfs(splitter, &pdf);
}

fn counts_pages_and_extracts_ranges(splitter: &impl PdfSplitter, pdf: &[u8]) {
    assert_eq!(splitter.page_count(pdf).unwrap(), 7);
    let part = splitter
        .extract(
            pdf,
            PageRange {
                start_page: 3,
                end_page: 5,
            },
        )
        .unwrap();
    assert_eq!(splitter.page_count(&part).unwrap(), 3);
    let last = splitter
        .extract(
            pdf,
            PageRange {
                start_page: 7,
                end_page: 7,
            },
        )
        .unwrap();
    assert_eq!(splitter.page_count(&last).unwrap(), 1);
}

fn refuses_ranges_outside_and_non_pdfs(splitter: &impl PdfSplitter, pdf: &[u8]) {
    let past_end = PageRange {
        start_page: 6,
        end_page: 8,
    };
    assert_code(splitter.extract(pdf, past_end), ErrorCode::UploadInvalid);
    let page_zero = PageRange {
        start_page: 0,
        end_page: 1,
    };
    assert_code(splitter.extract(pdf, page_zero), ErrorCode::UploadInvalid);
    assert_code(splitter.page_count(b"hello"), ErrorCode::UploadInvalid);
}

/// Runs every `RemoteFiles` promise against a fresh store from `make`.
///
/// # Panics
/// On the first broken promise.
pub fn remote_files_contract<R: RemoteFiles>(make: impl Fn() -> R) {
    let remote = make();

    let first = remote.upload("a.pdf", b"%PDF-1.7 a").unwrap();
    let second = remote.upload("b.pdf", b"%PDF-1.7 b").unwrap();

    assert!(!first.is_empty() && !second.is_empty());
    assert_ne!(first, second);
    remote.delete(&[first, second, "file_unknown".into()]);
    remote.delete(&[]);
}
