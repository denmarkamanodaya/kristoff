//! `LopdfSplitter` keeps the `PdfSplitter` contract, and drives core's processing end to end on
//! real PDFs made with lopdf.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::domain::parts::{PageRange, SplitLimits};
use kristoff_core::error::ErrorCode;
use kristoff_core::model::DocStatus;
use kristoff_core::ports::{DocStore, FileStore, PdfSplitter};
use kristoff_core::usecases::processing::{ProcessingDeps, process_doc};
use kristoff_pdf::LopdfSplitter;
use kristoff_testkit::builders::doc;
use kristoff_testkit::contract::{assert_code, pdf_splitter_contract};
use kristoff_testkit::pdf::{first_page_text, make_pdf};
use kristoff_testkit::world::World;

const D: &str = "00000000000000d1";

#[test]
fn lopdf_keeps_the_pdf_splitter_contract() {
    pdf_splitter_contract(&LopdfSplitter, |pages| make_pdf(pages, |_| 0));
}

#[test]
fn a_part_keeps_only_its_own_pages_in_order() {
    let pdf = make_pdf(12, |_| 0);

    let part = LopdfSplitter
        .extract(
            &pdf,
            PageRange {
                start_page: 5,
                end_page: 9,
            },
        )
        .unwrap();

    assert_eq!(LopdfSplitter.page_count(&part).unwrap(), 5);
    assert!(first_page_text(&part).contains("PAGE 5"));
}

#[test]
fn refuses_bytes_that_are_not_a_pdf() {
    assert_code(
        LopdfSplitter.page_count(b"%PDF-1.7 truncated"),
        ErrorCode::UploadInvalid,
    );
}

fn process_with_lopdf(
    w: &World,
    pdf: &[u8],
    limits: SplitLimits,
) -> kristoff_core::error::AppResult<kristoff_core::model::Doc> {
    w.store.create_doc(&doc(D, DocStatus::Processing)).unwrap();
    w.files.write_partial(D, 0, pdf).unwrap();
    w.files.commit_partial(D).unwrap();
    let deps = ProcessingDeps {
        docs: &w.store,
        files: &w.files,
        splitter: &LopdfSplitter,
        remote: &w.remote,
        limits,
        key_configured: true,
    };
    process_doc(&deps, D)
}

#[test]
fn a_250_page_pdf_becomes_three_real_parts() {
    let w = World::new();

    let ready = process_with_lopdf(&w, &make_pdf(250, |_| 0), SplitLimits::CLAUDE).unwrap();

    let ranges: Vec<_> = ready
        .parts
        .iter()
        .map(|p| (p.start_page, p.end_page))
        .collect();
    assert_eq!(ranges, [(1, 100), (101, 200), (201, 250)]);
    for p in &ready.parts {
        let bytes = w.files.read_stored(D, Some(p.n)).unwrap();
        assert_eq!(
            LopdfSplitter.page_count(&bytes).unwrap(),
            p.end_page - p.start_page + 1
        );
        assert!(first_page_text(&bytes).contains(&format!("PAGE {}", p.start_page)));
    }
}

#[test]
fn heavy_pages_are_halved_by_the_real_part_size() {
    let w = World::new();
    let limits = SplitLimits {
        max_pages: 4,
        max_bytes: 25_000,
        max_parts: 200,
    };

    let ready = process_with_lopdf(&w, &make_pdf(8, |_| 10_000), limits).unwrap();

    let ranges: Vec<_> = ready
        .parts
        .iter()
        .map(|p| (p.start_page, p.end_page))
        .collect();
    assert_eq!(ranges, [(1, 2), (3, 4), (5, 6), (7, 8)]);
    assert!(ready.parts.iter().all(|p| p.size <= 25_000));
}

#[test]
fn one_page_over_the_limit_fails_the_doc() {
    let w = World::new();
    let limits = SplitLimits {
        max_pages: 100,
        max_bytes: 25_000,
        max_parts: 200,
    };

    let failed = process_with_lopdf(
        &w,
        &make_pdf(3, |p| if p == 2 { 40_000 } else { 100 }),
        limits,
    );

    assert_code(failed, ErrorCode::UploadInvalid);
    assert_eq!(w.store.get_doc(D).unwrap().value.status, DocStatus::Failed);
}
