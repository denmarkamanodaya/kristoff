//! Chunked upload and confirm: ported from `confirm-doc/v1.test.ts`, plus the desktop chunk rules.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::error::ErrorCode;
use kristoff_core::model::{Doc, DocStatus};
use kristoff_core::ports::{DocStore, ProcessedDoc};
use kristoff_core::usecases::docs::{DocUploadInput, create_doc};
use kristoff_core::usecases::upload::{Chunk, confirm_doc, upload_chunk};
use kristoff_testkit::builders::part;
use kristoff_testkit::contract::assert_code;
use kristoff_testkit::fakes::FakeProcessor;
use kristoff_testkit::world::World;

const PDF: &[u8] = b"%PDF-1.7 tiny body";

fn pending(world: &World, size: usize) -> Doc {
    let input = DocUploadInput {
        file_name: "Case.pdf".into(),
        size: size as u64,
    };
    create_doc(&world.library(), &input).unwrap().doc
}

fn chunk(offset: u64, bytes: &[u8]) -> Chunk<'_> {
    Chunk { offset, bytes }
}

#[test]
fn writes_chunks_in_order_and_reports_the_bytes_written() {
    let w = World::new();
    let doc = pending(&w, PDF.len());

    assert_eq!(
        upload_chunk(&w.upload(), &doc.id, chunk(0, &PDF[..8])).unwrap(),
        8
    );
    assert_eq!(
        upload_chunk(&w.upload(), &doc.id, chunk(8, &PDF[8..])).unwrap(),
        18
    );
    assert_eq!(
        upload_chunk(&w.upload(), &doc.id, chunk(8, &PDF[8..])).unwrap(),
        18
    );
}

#[test]
fn refuses_an_offset_past_the_bytes_written_so_far() {
    let w = World::new();
    let doc = pending(&w, PDF.len());

    assert_code(
        upload_chunk(&w.upload(), &doc.id, chunk(4, &PDF[4..])),
        ErrorCode::UploadInvalid,
    );
}

#[test]
fn refuses_a_chunk_that_ends_past_the_declared_size() {
    let w = World::new();
    let doc = pending(&w, 10);

    assert_code(
        upload_chunk(&w.upload(), &doc.id, chunk(0, PDF)),
        ErrorCode::UploadInvalid,
    );
}

#[test]
fn refuses_an_empty_chunk_and_an_unknown_doc() {
    let w = World::new();
    let doc = pending(&w, PDF.len());

    assert_code(
        upload_chunk(&w.upload(), &doc.id, chunk(0, b"")),
        ErrorCode::UploadInvalid,
    );
    assert_code(
        upload_chunk(&w.upload(), "ffffffffffffffff", chunk(0, PDF)),
        ErrorCode::DocNotFound,
    );
}

#[test]
fn confirms_a_complete_pdf_as_ready_and_commits_its_bytes() {
    let w = World::new();
    let doc = pending(&w, PDF.len());
    upload_chunk(&w.upload(), &doc.id, chunk(0, PDF)).unwrap();

    let ready = confirm_doc(&w.upload(), &doc.id).unwrap();

    assert_eq!(
        (ready.status, ready.page_count, ready.parts.len()),
        (DocStatus::Ready, None, 0)
    );
    assert_eq!(w.files.stored(&doc.id).as_deref(), Some(PDF));
    assert_eq!(w.processor.seen(), std::slice::from_ref(&doc.id));
    assert_code(
        upload_chunk(&w.upload(), &doc.id, chunk(0, PDF)),
        ErrorCode::UploadInvalid,
    );
}

#[test]
fn stores_what_the_processor_found() {
    let mut w = World::new();
    let processed = ProcessedDoc {
        page_count: Some(150),
        parts: vec![part(1), part(2)],
    };
    w.processor = FakeProcessor::returning(Ok(processed));
    let doc = pending(&w, PDF.len());
    upload_chunk(&w.upload(), &doc.id, chunk(0, PDF)).unwrap();

    let ready = confirm_doc(&w.upload(), &doc.id).unwrap();

    assert_eq!(
        (ready.page_count, ready.parts),
        (Some(150), vec![part(1), part(2)])
    );
}

#[test]
fn refuses_a_doc_with_no_upload() {
    let w = World::new();
    let doc = pending(&w, PDF.len());

    assert_code(confirm_doc(&w.upload(), &doc.id), ErrorCode::UploadInvalid);
}

#[test]
fn refuses_a_size_mismatch_and_keeps_the_doc_pending() {
    let w = World::new();
    let doc = pending(&w, PDF.len());
    upload_chunk(&w.upload(), &doc.id, chunk(0, &PDF[..10])).unwrap();

    assert_code(confirm_doc(&w.upload(), &doc.id), ErrorCode::UploadInvalid);
    assert_eq!(
        w.store.get_doc(&doc.id).unwrap().value.status,
        DocStatus::Pending
    );
}

#[test]
fn refuses_bytes_that_do_not_start_with_the_pdf_magic() {
    let w = World::new();
    let fake = b"%PDX-1.7 tiny body";
    let doc = pending(&w, fake.len());
    upload_chunk(&w.upload(), &doc.id, chunk(0, fake)).unwrap();

    assert_code(confirm_doc(&w.upload(), &doc.id), ErrorCode::UploadInvalid);
    assert_eq!(w.processor.seen(), Vec::<String>::new());
}

#[test]
fn returns_a_confirmed_doc_unchanged_without_processing_it_again() {
    let w = World::new();
    let doc = pending(&w, PDF.len());
    upload_chunk(&w.upload(), &doc.id, chunk(0, PDF)).unwrap();
    let first = confirm_doc(&w.upload(), &doc.id).unwrap();

    let again = confirm_doc(&w.upload(), &doc.id).unwrap();

    assert_eq!(again, first);
    assert_eq!(w.processor.seen().len(), 1);
}

#[test]
fn passes_a_processor_failure_through_and_marks_the_doc_failed() {
    let mut w = World::new();
    let failure = kristoff_core::error::AppError::new(ErrorCode::UploadInvalid, "unreadable");
    w.processor = FakeProcessor::returning(Err(failure));
    let doc = pending(&w, PDF.len());
    upload_chunk(&w.upload(), &doc.id, chunk(0, PDF)).unwrap();

    assert_code(confirm_doc(&w.upload(), &doc.id), ErrorCode::UploadInvalid);
    assert_eq!(
        w.store.get_doc(&doc.id).unwrap().value.status,
        DocStatus::Failed
    );
}
