//! The disk store's layout and its refusal of anything but a doc id.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_adapters::disk_file_store::DiskFileStore;
use kristoff_core::error::ErrorCode;
use kristoff_core::ports::FileStore;

const ID: &str = "0123456789abcdef";

#[test]
fn purge_removes_the_pdf_the_parts_folder_and_the_upload() {
    let dir = tempfile::tempdir().unwrap();
    let store = DiskFileStore::open(dir.path()).unwrap();
    store.write_partial(ID, 0, b"%PDF-1").unwrap();
    store.commit_partial(ID).unwrap();
    std::fs::create_dir_all(dir.path().join(ID)).unwrap();
    std::fs::write(store.part_path(ID, 1).unwrap(), b"%PDF-part").unwrap();
    store.write_partial(ID, 0, b"%PDF-2").unwrap();

    store.purge(ID).unwrap();

    let left: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert!(left.is_empty(), "{left:?}");
}

#[test]
fn refuses_a_doc_id_that_could_name_another_path() {
    let dir = tempfile::tempdir().unwrap();
    let store = DiskFileStore::open(dir.path()).unwrap();

    let err = store
        .write_partial("../../etc/passwd", 0, b"x")
        .unwrap_err();

    assert_eq!(err.code, ErrorCode::ValidationFailed);
}
