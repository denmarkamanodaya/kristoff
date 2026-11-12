//! `AuthStateStore` contract.

use super::assert_code;
use kristoff_core::error::ErrorCode;
use kristoff_core::model::TotpState;
use kristoff_core::ports::AuthStateStore;

/// Runs every `AuthStateStore` promise against fresh stores from `make`.
pub fn auth_state_store_contract<S: AuthStateStore>(make: impl Fn() -> S) {
    starts_at_version_zero_with_the_default_state(&make());
    accepts_only_a_later_step(&make());
    records_a_failure_at_the_read_version_only(&make());
    an_accepted_step_clears_failures_and_lock(&make());
}

fn starts_at_version_zero_with_the_default_state(store: &impl AuthStateStore) {
    let read = store.get_auth_state().unwrap();

    assert_eq!((read.value, read.version), (TotpState::default(), 0));
}

fn accepts_only_a_later_step(store: &impl AuthStateStore) {
    store.accept_step(100).unwrap();

    assert_code(store.accept_step(100), ErrorCode::OtpReplayed);
    assert_code(store.accept_step(99), ErrorCode::OtpReplayed);
    store.accept_step(101).unwrap();
    assert_eq!(store.get_auth_state().unwrap().value.last_step, 101);
}

fn records_a_failure_at_the_read_version_only(store: &impl AuthStateStore) {
    let read = store.get_auth_state().unwrap();
    let next = TotpState {
        last_step: 0,
        fail_count: 1,
        locked_until_ms: 0,
    };

    store.record_failure(&next, read.version).unwrap();

    let after = store.get_auth_state().unwrap();
    assert_eq!(after.value, next);
    assert!(after.version > read.version);
    assert_code(
        store.record_failure(&next, read.version),
        ErrorCode::VersionConflict,
    );
}

fn an_accepted_step_clears_failures_and_lock(store: &impl AuthStateStore) {
    let locked = TotpState {
        last_step: 5,
        fail_count: 3,
        locked_until_ms: 9,
    };
    store.record_failure(&locked, 0).unwrap();

    store.accept_step(6).unwrap();

    let value = store.get_auth_state().unwrap().value;
    assert_eq!(
        value,
        TotpState {
            last_step: 6,
            fail_count: 0,
            locked_until_ms: 0
        }
    );
}
