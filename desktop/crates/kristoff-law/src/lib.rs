//! PH law index adapter: the read-only `phlaws.db` schema, the int8 vector format and
//! [`LawIndex`], which implements `kristoff_core::ports::LawSearch`.
//! Plan: docs/desktop/phases/phase-3-law-index.md. Citation parsing and provision stitching
//! are pure rules in `kristoff_core::domain::law`.

pub mod chunk_kind;
mod embedder;
mod error;
pub mod fts;
pub mod fusion;
mod index;
pub mod quant;
pub mod schema;

pub use error::LawIndexError;
pub use index::{LawIndex, QueryEmbedder};
