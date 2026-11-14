//! `MemoryStore`: one in-memory fake of the doc, chat, project, and usage ports, the
//! same shape as the SQLite adapter (one struct, several ports).

use kristoff_core::domain::cursor::{CursorPosition, decode_cursor, page_of};
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::model::{Chat, Doc, Message, Page, Project, Usage, Versioned};
use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Everything the fake stores.
#[derive(Debug, Default)]
pub(super) struct State {
    pub docs: BTreeMap<String, Versioned<Doc>>,
    pub chats: BTreeMap<String, Versioned<Chat>>,
    pub messages: Vec<(String, Message)>,
    pub projects: BTreeMap<String, Versioned<Project>>,
    pub days: BTreeMap<String, (u32, Usage)>,
    pub total: Usage,
    pub daily_question_cap: Option<u32>,
}

/// The in-memory store.
#[derive(Debug, Default)]
pub struct MemoryStore(Mutex<State>);

impl MemoryStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub(super) fn state(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// One page of `items` (already sorted by `(key, id)` descending) after `cursor`.
pub(super) fn page_desc<T: Clone>(
    items: &[T],
    cursor: Option<&str>,
    limit: usize,
    position: impl Fn(&T) -> CursorPosition,
) -> AppResult<Page<T>> {
    let after = cursor.map(decode_cursor).transpose()?;
    let rest: Vec<&T> = items
        .iter()
        .filter(|item| {
            after.as_ref().is_none_or(|a| {
                let p = position(item);
                (p.key, p.id.as_str()) < (a.key, a.id.as_str())
            })
        })
        .collect();
    let rest: Vec<T> = rest.into_iter().take(limit + 1).cloned().collect();
    Ok(page_of(rest, limit, position))
}

/// The error a duplicate id gets.
pub(super) fn duplicate(what: &str) -> AppError {
    AppError::new(ErrorCode::Internal, format!("{what} already exists"))
}

/// The error a stale write gets.
pub(super) fn conflict() -> AppError {
    AppError::new(
        ErrorCode::VersionConflict,
        "the item changed since it was read",
    )
}
