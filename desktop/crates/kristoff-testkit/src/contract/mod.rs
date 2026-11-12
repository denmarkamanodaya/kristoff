//! One contract suite per driven port. Each takes a factory for a fresh, empty adapter and panics
//! (fails the calling test) on the first broken promise.

mod answer;
mod cases;
mod chats;
mod docs;
mod files;
mod law;
mod processing;
mod projects;
mod usage;

pub use answer::{
    FollowUpReply, answer_model_contract, follow_up_suggester_contract, sample_request,
    sample_script, tool_use_script,
};
pub use cases::{KnownCase, SAMPLE_CASE, case_search_contract};
pub use chats::chat_store_contract;
pub use docs::doc_store_contract;
pub use files::file_store_contract;
pub use law::law_search_contract;
pub use processing::{pdf_splitter_contract, remote_files_contract};
pub use projects::project_store_contract;
pub use usage::usage_meter_contract;

use kristoff_core::error::{AppResult, ErrorCode};

/// Asserts that `result` failed with `code`.
///
/// # Panics
/// When `result` is `Ok` or failed with another code.
#[track_caller]
pub fn assert_code<T: std::fmt::Debug>(result: AppResult<T>, code: ErrorCode) {
    match result {
        Err(e) => assert_eq!(e.code, code, "{e}"),
        Ok(v) => panic!("expected {}, got Ok({v:?})", code.as_str()),
    }
}
