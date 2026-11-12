//! PH law index adapters: the read-only `phlaws.db` and `phcases.db` schemas, the int8 vector
//! format, [`LawIndex`] (`kristoff_core::ports::LawSearch`) and [`CaseIndex`]
//! (`kristoff_core::ports::CaseSearch`).
//! Plan: docs/desktop/phases/phase-3-law-index.md. Citation parsing and provision stitching
//! are pure rules in `kristoff_core::domain::law`.

pub mod case_schema;
mod cases;
pub mod chunk_kind;
pub mod docket;
mod embedder;
mod error;
pub mod fts;
pub mod fusion;
mod index;
pub mod quant;
pub mod schema;
mod store;

pub use cases::CaseIndex;
pub use error::LawIndexError;
pub use index::{LawIndex, QueryEmbedder};
