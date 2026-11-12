//! Failures inside the law index and their mapping to the core error type.

use std::fmt;

use kristoff_core::error::{AppError, ErrorCode};

/// Failure inside the law index.
#[derive(Debug)]
pub enum LawIndexError {
    /// `SQLite` failed (open, query, or a malformed db).
    Db(rusqlite::Error),
    /// The query embedder failed.
    Embed(String),
    /// A stored vector has the wrong width.
    BadVector(i64),
    /// A lock was poisoned by a panic in another thread.
    Poisoned,
}

impl fmt::Display for LawIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Db(e) => write!(f, "LAW_DB: {e}"),
            Self::Embed(e) => write!(f, "LAW_EMBED: {e}"),
            Self::BadVector(id) => write!(f, "LAW_BAD_VECTOR: chunk {id}"),
            Self::Poisoned => write!(f, "LAW_POISONED"),
        }
    }
}

impl std::error::Error for LawIndexError {}

impl From<rusqlite::Error> for LawIndexError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Db(e)
    }
}

impl From<LawIndexError> for AppError {
    /// Every law index failure is `INTERNAL`: the index is bundled and local, so there is no
    /// caller action beyond reporting it. The message keeps the `LAW_*` cause for logs.
    fn from(e: LawIndexError) -> Self {
        AppError::new(ErrorCode::Internal, e.to_string())
    }
}
