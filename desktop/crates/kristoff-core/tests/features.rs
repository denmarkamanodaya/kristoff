//! The config features (phase-4 §4, phase-7 "Contract"): what each index turns on, and when
//! `lawReady` lets the launch screen go.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::usecases::settings::features;
use kristoff_testkit::fakes::{MemoryCaseSearch, MemoryLawSearch};

#[test]
fn a_case_index_alone_still_waits_for_its_warm_up() {
    let cases = MemoryCaseSearch::sample();

    let cold = features(None, Some(&cases), false);
    let warm = features(None, Some(&cases), true);

    assert!(!cold.law_ready && warm.law_ready);
    assert!(cold.cases && !cold.statutes);
    assert_eq!(cold.case_count, Some(3));
}

#[test]
fn with_no_index_nothing_is_offered_and_nothing_is_awaited() {
    let f = features(None, None, false);

    assert!(f.law_ready && !f.statutes && !f.cases);
    assert_eq!((f.law_count, f.case_count), (None, None));
}

#[test]
fn both_indexes_are_offered_together() {
    let (law, cases) = (MemoryLawSearch::sample(), MemoryCaseSearch::sample());

    let f = features(Some(&law), Some(&cases), true);

    assert!(f.statutes && f.cases && f.law_ready);
}
