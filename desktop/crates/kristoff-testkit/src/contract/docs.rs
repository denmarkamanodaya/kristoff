//! `DocStore` contract.

use super::assert_code;
use crate::builders::{doc, part};
use kristoff_core::error::ErrorCode;
use kristoff_core::model::DocStatus;
use kristoff_core::ports::{DocPatch, DocStore};

const A: &str = "0000000000000001";
const B: &str = "0000000000000002";
const C: &str = "0000000000000003";

/// Runs every `DocStore` promise against fresh stores from `make`.
pub fn doc_store_contract<S: DocStore>(make: impl Fn() -> S) {
    creates_reads_and_refuses_duplicates(&make());
    lists_newest_first_in_pages(&make());
    updates_at_the_read_version_only(&make());
    reads_many_and_removes(&make());
}

fn creates_reads_and_refuses_duplicates(store: &impl DocStore) {
    let mut d = doc(A, DocStatus::Ready);
    d.parts = vec![part(1), part(2)];
    d.page_count = Some(200);
    store.create_doc(&d).unwrap();

    let read = store.get_doc(A).unwrap();

    assert_eq!(read.value, d);
    assert_code(store.create_doc(&d), ErrorCode::Internal);
    assert_code(store.get_doc(B), ErrorCode::DocNotFound);
}

fn lists_newest_first_in_pages(store: &impl DocStore) {
    for id in [A, C, B] {
        store.create_doc(&doc(id, DocStatus::Pending)).unwrap();
    }

    let first = store.list_docs(None, 2).unwrap();
    let cursor = first.next_cursor.clone().expect("a next page");
    let second = store.list_docs(Some(&cursor), 2).unwrap();

    let ids = |p: &kristoff_core::model::Page<_>| -> Vec<String> {
        p.items
            .iter()
            .map(|d: &kristoff_core::model::Doc| d.id.clone())
            .collect()
    };
    assert_eq!(ids(&first), [C, B]);
    assert_eq!(ids(&second), [A]);
    assert_eq!(second.next_cursor, None);
    assert_code(
        store.list_docs(Some("not-a-cursor"), 2),
        ErrorCode::ValidationFailed,
    );
}

fn updates_at_the_read_version_only(store: &impl DocStore) {
    store.create_doc(&doc(A, DocStatus::Pending)).unwrap();
    let read = store.get_doc(A).unwrap();
    let patch = DocPatch {
        title: Some("Renamed".into()),
        status: Some(DocStatus::Ready),
        page_count: Some(12),
        parts: Some(vec![part(1)]),
    };

    let updated = store.update_doc(A, &patch, read.version).unwrap();

    assert_eq!(updated.title, "Renamed");
    assert_eq!(updated.status, DocStatus::Ready);
    assert_eq!((updated.page_count, updated.parts.len()), (Some(12), 1));
    assert_eq!(store.get_doc(A).unwrap().value, updated);
    assert_code(
        store.update_doc(A, &patch, read.version),
        ErrorCode::VersionConflict,
    );
    assert_code(store.update_doc(B, &patch, 1), ErrorCode::DocNotFound);
}

fn reads_many_and_removes(store: &impl DocStore) {
    store.create_doc(&doc(A, DocStatus::Ready)).unwrap();
    store.create_doc(&doc(B, DocStatus::Ready)).unwrap();

    let many = store.get_docs(&[A.into(), C.into(), B.into()]).unwrap();
    store.remove_doc(A).unwrap();

    let mut ids: Vec<_> = many.iter().map(|d| d.id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(ids, [A, B]);
    assert_code(store.get_doc(A), ErrorCode::DocNotFound);
    store.remove_doc(A).unwrap();
}
