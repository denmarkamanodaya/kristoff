//! The real adapters keep every port contract (the same suites the in-memory fakes pass).

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_adapters::disk_file_store::DiskFileStore;
use kristoff_adapters::sqlite::SqliteStore;
use kristoff_core::ports::ProjectStore;
use kristoff_testkit::builders::project;
use kristoff_testkit::contract::{
    chat_store_contract, doc_store_contract, file_store_contract, project_store_contract,
    usage_meter_contract,
};

fn memory() -> SqliteStore {
    SqliteStore::open_in_memory().unwrap()
}

/// A store holding the project the chat contract moves chats into (chats reference projects).
fn with_project() -> SqliteStore {
    let store = memory();
    store
        .create_project(&project("00000000000000f1", "Torts"), 50)
        .unwrap();
    store
}

#[test]
fn sqlite_keeps_the_doc_store_contract() {
    doc_store_contract(memory);
}

#[test]
fn sqlite_keeps_the_chat_store_contract() {
    chat_store_contract(with_project);
}

#[test]
fn sqlite_keeps_the_project_store_contract() {
    project_store_contract(memory);
}

#[test]
fn sqlite_keeps_the_usage_meter_contract() {
    usage_meter_contract(memory);
}

#[test]
fn sqlite_on_a_file_survives_a_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("kristoff.db");
    SqliteStore::open(&path)
        .unwrap()
        .create_project(&project("0000000000000001", "A"), 50)
        .unwrap();

    let reopened = SqliteStore::open(&path).unwrap();

    assert_eq!(reopened.list_projects().unwrap().len(), 1);
}

#[test]
fn disk_keeps_the_file_store_contract() {
    let dirs = std::cell::RefCell::new(Vec::new());
    let make = || {
        let dir = tempfile::tempdir().unwrap();
        let store = DiskFileStore::open(&dir.path().join("docs")).unwrap();
        dirs.borrow_mut().push(dir);
        store
    };
    file_store_contract(make, DiskFileStore::stored);
}
