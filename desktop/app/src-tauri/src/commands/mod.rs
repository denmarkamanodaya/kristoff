//! IPC commands: the driving adapters. Each module pairs plain functions (argument checks, one use case, shaping the result; testable without Tauri) with the `#[tauri::command]`s
//! that run them on a blocking thread. Command names: `kristoff_contracts::ipc::COMMANDS`.

// Tauri deserialises each command argument into an owned value and hands `State` by value, so
// by-value parameters are the macro's contract, not a choice.
#![allow(clippy::needless_pass_by_value)]

pub mod ask;
pub mod chats;
pub mod docs;
pub mod keys;
pub mod projects;
pub mod session;
pub mod upload;
pub mod viewer;

use crate::composition::Composition;
use kristoff_contracts::error::ErrorDto;
use kristoff_core::error::{AppError, ErrorCode};
use std::sync::Arc;
use tauri::State;

/// The composition as Tauri manages it.
pub type Shared = Arc<Composition>;

/// What every command returns over IPC.
pub type CommandResult<T> = Result<T, ErrorDto>;

/// Runs `work` with the composition on a blocking thread, so no command waits on SQLite or the
/// disk on the main thread; a panicked or cancelled thread answers `INTERNAL`.
pub(crate) async fn blocking<T: Send + 'static>(
    app: &State<'_, Shared>,
    work: impl FnOnce(&Composition) -> CommandResult<T> + Send + 'static,
) -> CommandResult<T> {
    let app = Arc::clone(app.inner());
    tauri::async_runtime::spawn_blocking(move || tag_failure(&app, work(&app)))
        .await
        .map_err(|e| ErrorDto::from(AppError::new(ErrorCode::Internal, e.to_string())))?
}

/// Tags a failed command with a fresh local request id, so a report can name the failure.
///
/// # Errors
/// `result`'s error, with its `requestId` set.
pub fn tag_failure<T>(app: &Composition, result: CommandResult<T>) -> CommandResult<T> {
    result.map_err(|e| e.with_request_id(app.request_id()))
}
