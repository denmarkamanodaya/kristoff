//! Test support: in-memory fakes of every driven port, builders, and one contract suite per port.
//! Each suite takes an adapter factory and runs against the fake (here) and the real adapter
//! (`kristoff-adapters` tests), so the fakes are trusted only because both pass.

// Test support: a failed setup or a broken contract must fail the calling test loudly, the same
// way `assert!` does, so unwrap/expect/panic are the intended behaviour in this crate only.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod builders;
pub mod contract;
pub mod fakes;
pub mod pdf;
pub mod world;
