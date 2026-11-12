//! The in-memory fakes keep every port contract (the same suites run on the real adapters).

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_testkit::contract::{
    auth_state_store_contract, chat_store_contract, doc_store_contract, file_store_contract,
    project_store_contract, usage_meter_contract,
};
use kristoff_testkit::fakes::{MemoryFiles, MemoryStore};

#[test]
fn memory_store_keeps_the_doc_store_contract() {
    doc_store_contract(MemoryStore::new);
}

#[test]
fn memory_store_keeps_the_chat_store_contract() {
    chat_store_contract(MemoryStore::new);
}

#[test]
fn memory_store_keeps_the_project_store_contract() {
    project_store_contract(MemoryStore::new);
}

#[test]
fn memory_store_keeps_the_usage_meter_contract() {
    usage_meter_contract(MemoryStore::new);
}

#[test]
fn memory_store_keeps_the_auth_state_store_contract() {
    auth_state_store_contract(MemoryStore::new);
}

#[test]
fn memory_files_keep_the_file_store_contract() {
    file_store_contract(MemoryFiles::new, MemoryFiles::stored);
}
