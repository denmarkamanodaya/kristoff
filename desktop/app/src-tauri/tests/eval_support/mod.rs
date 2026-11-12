//! Phase 5 eval support (docs/desktop/phases/phase-5-eval-package.md §1): the question set, a
//! recording law index, the uncited-statutory-claim check, scoring and cost. The runner is
//! `tests/eval.rs` (ignored; it calls the real API); the pure checks are tested in
//! `tests/eval_checks.rs`, which CI runs.
#![allow(dead_code)] // each test binary uses a different subset

pub mod claims;
pub mod questions;
pub mod recorder;
pub mod score;
