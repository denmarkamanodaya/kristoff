//! The bring-your-own key and the daily question limit (Phase 8).

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::error::ErrorCode;
use kristoff_core::ports::{SecretStore, SettingsStore};
use kristoff_core::usecases::keys::{
    KeyStatus, daily_question_cap, key_status, remove_key, save_key, update_daily_question_cap,
};
use kristoff_testkit::contract::assert_code;
use kristoff_testkit::fakes::{FakeKeyVerifier, MemorySecretStore, MemoryStore};

const GOOD: &str = "sk-ant-api03-goodkeyAB12";

#[test]
fn reports_no_key_when_none_is_saved() {
    let store = MemorySecretStore::new();

    let status = key_status(&store).unwrap();

    assert_eq!(
        status,
        KeyStatus {
            configured: false,
            hint: None
        }
    );
}

#[test]
fn saves_a_key_anthropic_accepts_and_shows_only_its_last_four_characters() {
    let store = MemorySecretStore::new();
    let verifier = FakeKeyVerifier::accepting(GOOD);

    let status = save_key(&store, &verifier, &format!("  {GOOD}\n")).unwrap();

    assert_eq!(
        status,
        KeyStatus {
            configured: true,
            hint: Some("AB12".into())
        }
    );
    assert_eq!(store.get_key().unwrap().as_deref(), Some(GOOD));
    assert_eq!(verifier.checked(), [GOOD]);
}

#[test]
fn rejects_a_malformed_key_without_calling_anthropic() {
    let store = MemorySecretStore::new();
    let verifier = FakeKeyVerifier::accepting(GOOD);

    for bad in [
        "",
        "sk-ant-",
        "sk-proj-abc123",
        "sk-ant-has space",
        &format!("sk-ant-{}", "x".repeat(250)),
    ] {
        assert_code(save_key(&store, &verifier, bad), ErrorCode::KeyInvalid);
    }

    assert_eq!(verifier.checked(), Vec::<String>::new());
    assert_eq!(store.get_key().unwrap(), None);
}

#[test]
fn keeps_the_old_key_when_anthropic_rejects_the_new_one() {
    let store = MemorySecretStore::holding(GOOD);
    let verifier = FakeKeyVerifier::accepting(GOOD);

    let result = save_key(&store, &verifier, "sk-ant-api03-revokedKEY9");

    assert_code(result, ErrorCode::KeyInvalid);
    assert_eq!(store.get_key().unwrap().as_deref(), Some(GOOD));
}

#[test]
fn saves_nothing_when_anthropic_cannot_be_reached() {
    let store = MemorySecretStore::new();

    let result = save_key(&store, &FakeKeyVerifier::unreachable(), GOOD);

    assert_code(result, ErrorCode::UpstreamUnavailable);
    assert_eq!(store.get_key().unwrap(), None);
}

#[test]
fn removes_the_saved_key() {
    let store = MemorySecretStore::holding(GOOD);

    let status = remove_key(&store).unwrap();

    assert_eq!(
        status,
        KeyStatus {
            configured: false,
            hint: None
        }
    );
    assert_eq!(store.get_key().unwrap(), None);
}

#[test]
fn uses_the_default_daily_limit_until_the_user_sets_one() {
    let settings = MemoryStore::new();

    assert_eq!(daily_question_cap(&settings, 100).unwrap(), 100);
    update_daily_question_cap(&settings, 25).unwrap();

    assert_eq!(daily_question_cap(&settings, 100).unwrap(), 25);
}

#[test]
fn accepts_daily_limits_from_one_to_ten_thousand_only() {
    let settings = MemoryStore::new();

    assert_code(
        update_daily_question_cap(&settings, 0),
        ErrorCode::ValidationFailed,
    );
    assert_code(
        update_daily_question_cap(&settings, 10_001),
        ErrorCode::ValidationFailed,
    );
    assert_eq!(update_daily_question_cap(&settings, 1).unwrap(), 1);
    assert_eq!(
        update_daily_question_cap(&settings, 10_000).unwrap(),
        10_000
    );
    assert_eq!(settings.daily_question_cap().unwrap(), Some(10_000));
}
