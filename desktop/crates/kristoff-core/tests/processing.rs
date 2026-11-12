//! Document processing: count, split, upload to the Files API, then ready (or failed).

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::domain::parts::SplitLimits;
use kristoff_core::error::ErrorCode;
use kristoff_core::model::{Doc, DocStatus};
use kristoff_core::ports::{DocStore, FileStore};
use kristoff_core::usecases::docs::delete_doc;
use kristoff_core::usecases::processing::{docs_to_resume, process_doc};
use kristoff_testkit::builders::{doc, part, uploaded_doc};
use kristoff_testkit::contract::assert_code;
use kristoff_testkit::fakes::fake_pdf;
use kristoff_testkit::world::World;

const D: &str = "00000000000000d1";

/// A confirmed doc in `processing` whose stored PDF is `pdf`.
fn confirmed(w: &World, pdf: &[u8]) -> Doc {
    let d = doc(D, DocStatus::Processing);
    w.store.create_doc(&d).unwrap();
    w.files.write_partial(D, 0, pdf).unwrap();
    w.files.commit_partial(D).unwrap();
    d
}

fn status(w: &World) -> DocStatus {
    w.store.get_doc(D).unwrap().value.status
}

#[test]
fn a_small_pdf_is_uploaded_whole_and_becomes_ready() {
    let w = World::new();
    confirmed(&w, &fake_pdf(&[10; 12]));

    let ready = process_doc(&w.processing(), D).unwrap();

    assert_eq!(
        (ready.status, ready.page_count, ready.parts.len()),
        (DocStatus::Ready, Some(12), 0)
    );
    let remote = w.remote.stored();
    assert_eq!(ready.file_id.as_deref(), Some("file_mem_1"));
    assert_eq!(remote["file_mem_1"].0, format!("{D}.pdf"));
    assert_eq!(w.store.get_doc(D).unwrap().value, ready);
}

#[test]
fn a_250_page_pdf_becomes_three_stored_and_uploaded_parts() {
    let w = World::new();
    confirmed(&w, &fake_pdf(&[10; 250]));

    let ready = process_doc(&w.processing(), D).unwrap();

    let ranges: Vec<_> = ready
        .parts
        .iter()
        .map(|p| (p.n, p.start_page, p.end_page))
        .collect();
    assert_eq!(ranges, [(1, 1, 100), (2, 101, 200), (3, 201, 250)]);
    assert_eq!(ready.file_id, None);
    for p in &ready.parts {
        let stored = w.files.read_stored(D, Some(p.n)).unwrap();
        assert_eq!(p.size, stored.len() as u64);
        let id = p.file_id.clone().unwrap();
        assert_eq!(
            w.remote.stored()[&id],
            (format!("{D}-part-{}.pdf", p.n), stored.len())
        );
    }
}

#[test]
fn a_part_over_the_byte_limit_is_halved_by_its_real_size() {
    let mut w = World::new();
    w.limits = SplitLimits {
        max_pages: 4,
        max_bytes: 250,
        max_parts: 200,
    };
    confirmed(&w, &fake_pdf(&[100; 8]));

    let ready = process_doc(&w.processing(), D).unwrap();

    let ranges: Vec<_> = ready
        .parts
        .iter()
        .map(|p| (p.start_page, p.end_page))
        .collect();
    assert_eq!(ranges, [(1, 2), (3, 4), (5, 6), (7, 8)]);
    assert!(ready.parts.iter().all(|p| p.size <= 250));
}

#[test]
fn a_single_page_over_the_limit_fails_the_doc() {
    let mut w = World::new();
    w.limits = SplitLimits {
        max_pages: 100,
        max_bytes: 250,
        max_parts: 200,
    };
    confirmed(&w, &fake_pdf(&[10, 400, 10]));

    assert_code(process_doc(&w.processing(), D), ErrorCode::UploadInvalid);
    assert_eq!(status(&w), DocStatus::Failed);
}

#[test]
fn more_parts_than_allowed_fails_the_doc() {
    let mut w = World::new();
    w.limits = SplitLimits {
        max_pages: 2,
        max_bytes: 1_000_000,
        max_parts: 3,
    };
    confirmed(&w, &fake_pdf(&[10; 7]));

    assert_code(process_doc(&w.processing(), D), ErrorCode::UploadInvalid);
    assert_eq!(status(&w), DocStatus::Failed);
}

#[test]
fn an_upload_failure_fails_the_doc_and_deletes_what_was_uploaded() {
    let w = World::new();
    w.remote.fail_from(2);
    confirmed(&w, &fake_pdf(&[10; 250]));

    assert_code(
        process_doc(&w.processing(), D),
        ErrorCode::UpstreamUnavailable,
    );

    assert_eq!(status(&w), DocStatus::Failed);
    assert_eq!(w.remote.deleted(), ["file_mem_1"]);
    assert!(w.remote.stored().is_empty());
}

#[test]
fn an_unreadable_pdf_fails_the_doc_without_uploading() {
    let w = World::new();
    confirmed(&w, b"%PDF-1.7 not the fake format");

    assert_code(process_doc(&w.processing(), D), ErrorCode::UploadInvalid);
    assert_eq!(status(&w), DocStatus::Failed);
    assert!(w.remote.stored().is_empty());
}

#[test]
fn a_doc_that_is_not_processing_is_left_alone() {
    let w = World::new();
    w.store.create_doc(&uploaded_doc(D)).unwrap();

    let same = process_doc(&w.processing(), D).unwrap();

    assert_eq!(same, uploaded_doc(D));
    assert!(w.remote.stored().is_empty());
}

#[test]
fn resumes_processing_docs_and_phase_1_ready_docs_without_file_ids() {
    let w = World::new();
    let ids = [
        "00000000000000a1",
        "00000000000000a2",
        "00000000000000a3",
        "00000000000000a4",
    ];
    w.store
        .create_doc(&doc(ids[0], DocStatus::Processing))
        .unwrap();
    w.store.create_doc(&doc(ids[1], DocStatus::Ready)).unwrap();
    w.store.create_doc(&uploaded_doc(ids[2])).unwrap();
    w.store.create_doc(&doc(ids[3], DocStatus::Failed)).unwrap();

    let mut resumed = docs_to_resume(&w.store).unwrap();

    resumed.sort_unstable();
    assert_eq!(resumed, [ids[0], ids[1]]);
    assert_eq!(
        w.store.get_doc(ids[1]).unwrap().value.status,
        DocStatus::Processing
    );
}

#[test]
fn deleting_a_doc_deletes_its_files_api_copies() {
    let w = World::new();
    let mut split = uploaded_doc(D);
    split.file_id = None;
    split.parts = vec![part(1), part(2)];
    w.store.create_doc(&split).unwrap();

    delete_doc(&w.library(), D).unwrap();

    assert_eq!(w.remote.deleted(), ["file_part_1", "file_part_2"]);
    assert_code(w.store.get_doc(D), ErrorCode::DocNotFound);
}
