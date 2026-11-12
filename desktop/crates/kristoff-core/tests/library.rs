//! Library use cases: ported from `create-doc-upload`, `rename-doc`, `get-doc-link` and
//! `delete-doc` tests.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::error::ErrorCode;
use kristoff_core::model::DocStatus;
use kristoff_core::ports::{ChatStore, DocStore, FileStore};
use kristoff_core::usecases::docs::{
    DocUploadInput, create_doc, delete_doc, get_doc_link, list_docs, rename_doc,
};
use kristoff_core::usecases::viewer::{doc_window, read_doc_file};
use kristoff_testkit::builders::{chat, doc, part};
use kristoff_testkit::contract::assert_code;
use kristoff_testkit::world::{START_MS, World};

const D: &str = "00000000000000d1";
const OTHER: &str = "00000000000000d2";

#[test]
fn creates_a_pending_doc_titled_from_the_file_name_with_its_upload_url() {
    let w = World::new();
    let input = DocUploadInput {
        file_name: "People v. Cruz.pdf".into(),
        size: 5_000_000,
    };

    let upload = create_doc(&w.library(), &input).unwrap();

    assert_eq!(upload.doc.title, "People v. Cruz");
    assert_eq!(
        (upload.doc.status, upload.doc.size),
        (DocStatus::Pending, 5_000_000)
    );
    assert_eq!(upload.doc.created_at_ms, START_MS);
    assert_eq!(
        upload.upload_url,
        format!("kristoff-upload://{}", upload.doc.id)
    );
    assert_eq!(w.store.get_doc(&upload.doc.id).unwrap().value, upload.doc);
}

#[test]
fn lists_the_library_newest_first() {
    let w = World::new();
    let first = create_doc(
        &w.library(),
        &DocUploadInput {
            file_name: "a".into(),
            size: 1,
        },
    );
    let second = create_doc(
        &w.library(),
        &DocUploadInput {
            file_name: "b".into(),
            size: 1,
        },
    );

    let page = list_docs(&w.library(), None).unwrap();

    let ids: Vec<_> = page.items.iter().map(|d| d.id.clone()).collect();
    assert_eq!(ids, [second.unwrap().doc.id, first.unwrap().doc.id]);
}

#[test]
fn renames_with_a_trimmed_title_and_refuses_a_blank_one() {
    let w = World::new();
    w.store.create_doc(&doc(D, DocStatus::Ready)).unwrap();

    assert_eq!(
        rename_doc(&w.library(), D, "  Torts digest ")
            .unwrap()
            .title,
        "Torts digest"
    );
    assert_code(
        rename_doc(&w.library(), D, "   "),
        ErrorCode::ValidationFailed,
    );
    assert_code(rename_doc(&w.library(), OTHER, "x"), ErrorCode::DocNotFound);
}

#[test]
fn links_a_ready_doc_and_its_parts_for_300_seconds() {
    let w = World::new();
    let mut split = doc(D, DocStatus::Ready);
    split.parts = vec![part(1), part(2)];
    w.store.create_doc(&split).unwrap();

    let whole = get_doc_link(&w.library(), D, None).unwrap();
    let second = get_doc_link(&w.library(), D, Some(2)).unwrap();

    assert_eq!(whole.url, format!("kristoff://doc/{D}"));
    assert_eq!(second.url, format!("kristoff://doc/{D}/part-2"));
    assert_eq!(whole.expires_at_ms, START_MS + 300_000);
    assert_code(
        get_doc_link(&w.library(), D, Some(3)),
        ErrorCode::DocNotFound,
    );
}

#[test]
fn refuses_a_link_to_a_doc_that_is_not_ready() {
    let w = World::new();
    w.store.create_doc(&doc(D, DocStatus::Pending)).unwrap();

    assert_code(get_doc_link(&w.library(), D, None), ErrorCode::DocNotFound);
}

#[test]
fn delete_strips_the_doc_and_its_parts_from_every_chat_keeping_updated_at() {
    let w = World::new();
    w.store.create_doc(&doc(D, DocStatus::Ready)).unwrap();
    let ids = [D, "00000000000000d1:p2", OTHER];
    w.store
        .create_chat(&chat("00000000000000c1", 10, &ids))
        .unwrap();
    w.store
        .create_chat(&chat("00000000000000c2", 20, &[OTHER]))
        .unwrap();

    delete_doc(&w.library(), D).unwrap();

    let c1 = w.store.get_chat("00000000000000c1").unwrap().value;
    let c2 = w.store.get_chat("00000000000000c2").unwrap().value;
    assert_eq!((c1.doc_ids, c1.updated_at_ms), (vec![OTHER.to_owned()], 10));
    assert_eq!(c2.doc_ids, [OTHER]);
}

#[test]
fn delete_purges_the_files_and_then_the_doc() {
    let w = World::new();
    w.store.create_doc(&doc(D, DocStatus::Pending)).unwrap();
    w.files.write_partial(D, 0, b"%PDF-").unwrap();

    delete_doc(&w.library(), D).unwrap();

    assert_eq!(w.files.partial_len(D).unwrap(), None);
    assert_code(w.store.get_doc(D), ErrorCode::DocNotFound);
    assert_code(delete_doc(&w.library(), D), ErrorCode::DocNotFound);
}

#[test]
fn opens_a_viewer_window_at_a_page_of_a_part_titled_with_the_doc() {
    let w = World::new();
    let mut split = doc(D, DocStatus::Ready);
    split.parts = vec![part(1), part(2)];
    w.store.create_doc(&split).unwrap();

    let window = doc_window(&w.store, D, Some(2), Some(7)).unwrap();
    let whole_ready = doc(OTHER, DocStatus::Ready);
    w.store.create_doc(&whole_ready).unwrap();
    let whole = doc_window(&w.store, OTHER, None, None).unwrap();

    assert_eq!(window.label, format!("doc-{D}-part-2"));
    assert_eq!(window.url, format!("kristoff://doc/{D}/part-2#page=7"));
    assert_eq!(window.title, format!("Doc {D} (part 2)"));
    assert_eq!(
        (whole.url, whole.title),
        (format!("kristoff://doc/{OTHER}"), format!("Doc {OTHER}"))
    );
}

#[test]
fn refuses_a_viewer_for_a_missing_unready_or_partless_doc_and_a_bad_page() {
    let w = World::new();
    w.store.create_doc(&doc(D, DocStatus::Ready)).unwrap();
    w.store.create_doc(&doc(OTHER, DocStatus::Pending)).unwrap();

    assert_code(
        doc_window(&w.store, "00000000000000ff", None, None),
        ErrorCode::DocNotFound,
    );
    assert_code(
        doc_window(&w.store, OTHER, None, None),
        ErrorCode::DocNotFound,
    );
    assert_code(
        doc_window(&w.store, D, Some(1), None),
        ErrorCode::DocNotFound,
    );
    assert_code(
        doc_window(&w.store, D, None, Some(0)),
        ErrorCode::ValidationFailed,
    );
}

#[test]
fn refuses_the_bytes_of_a_doc_that_is_not_ready_even_when_its_file_exists() {
    let w = World::new();
    w.store.create_doc(&doc(D, DocStatus::Processing)).unwrap();
    w.files.write_partial(D, 0, b"%PDF-whole").unwrap();
    w.files.commit_partial(D).unwrap();

    assert_code(
        read_doc_file(&w.store, &w.files, D, None),
        ErrorCode::DocNotFound,
    );
}

#[test]
fn serves_the_bytes_of_a_ready_doc_and_part_only() {
    let w = World::new();
    let mut split = doc(D, DocStatus::Ready);
    split.parts = vec![part(1)];
    w.store.create_doc(&split).unwrap();
    w.files.write_partial(D, 0, b"%PDF-whole").unwrap();
    w.files.commit_partial(D).unwrap();
    w.files.write_part(D, 1, b"%PDF-part1").unwrap();
    w.store.create_doc(&doc(OTHER, DocStatus::Pending)).unwrap();

    assert_eq!(
        read_doc_file(&w.store, &w.files, D, None).unwrap(),
        b"%PDF-whole"
    );
    assert_eq!(
        read_doc_file(&w.store, &w.files, D, Some(1)).unwrap(),
        b"%PDF-part1"
    );
    assert_code(
        read_doc_file(&w.store, &w.files, D, Some(2)),
        ErrorCode::DocNotFound,
    );
    assert_code(
        read_doc_file(&w.store, &w.files, OTHER, None),
        ErrorCode::DocNotFound,
    );
}
