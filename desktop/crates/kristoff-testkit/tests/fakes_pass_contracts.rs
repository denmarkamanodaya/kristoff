//! The in-memory fakes keep every port contract (the same suites run on the real adapters).

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::domain::model_request::FollowUps;
use kristoff_core::model::Usage;
use kristoff_testkit::contract::{
    FollowUpReply, answer_model_contract, chat_store_contract, doc_store_contract,
    file_store_contract, follow_up_suggester_contract, law_search_contract, pdf_splitter_contract,
    project_store_contract, remote_files_contract, usage_meter_contract,
};
use kristoff_testkit::fakes::{
    FakeSplitter, FixedFollowUps, MemoryFiles, MemoryLawSearch, MemoryRemoteFiles, MemoryStore,
    ScriptedModel, fake_pdf,
};

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
fn memory_files_keep_the_file_store_contract() {
    file_store_contract(MemoryFiles::new, MemoryFiles::stored);
}

#[test]
fn scripted_model_keeps_the_answer_model_contract() {
    answer_model_contract(|script| ScriptedModel::new(script.clone()));
}

#[test]
fn fixed_follow_ups_keep_the_follow_up_suggester_contract() {
    follow_up_suggester_contract(|reply| {
        let questions = match reply {
            FollowUpReply::Valid(q) => q,
            FollowUpReply::CutOff(_) | FollowUpReply::Garbled => vec![],
        };
        FixedFollowUps::returning(FollowUps {
            questions,
            usage: Some(Usage::default()),
        })
    });
}

#[test]
fn fake_splitter_keeps_the_pdf_splitter_contract() {
    pdf_splitter_contract(&FakeSplitter, |pages| fake_pdf(&vec![10; pages as usize]));
}

#[test]
fn memory_remote_files_keep_the_remote_files_contract() {
    remote_files_contract(MemoryRemoteFiles::new);
}

#[test]
fn memory_law_search_keeps_the_law_search_contract() {
    law_search_contract(&MemoryLawSearch::sample());
}

#[test]
fn memory_case_search_keeps_the_case_search_contract() {
    kristoff_testkit::contract::case_search_contract(
        &kristoff_testkit::fakes::MemoryCaseSearch::sample(),
        &kristoff_testkit::contract::SAMPLE_CASE,
    );
}

#[test]
fn memory_secret_store_keeps_the_secret_store_contract() {
    kristoff_testkit::contract::secret_store_contract(
        kristoff_testkit::fakes::MemorySecretStore::new,
    );
}

#[test]
fn memory_store_keeps_the_settings_store_contract() {
    kristoff_testkit::contract::settings_store_contract(MemoryStore::new);
}
