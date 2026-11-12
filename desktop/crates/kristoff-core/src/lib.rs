//! Kristoff core: the domain, the use cases and the ports they speak through. Pure: no IO, no
//! clock, no env (its `clippy.toml` bans them). Plan: docs/desktop/phases/phase-1-core-storage.md §3.

pub mod domain;
pub mod error;
pub mod limits;
pub mod model;
pub mod ports;
pub mod usecases;
pub mod util;
