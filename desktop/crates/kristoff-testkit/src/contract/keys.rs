//! `SecretStore` and `SettingsStore` contracts (Phase 8).

use kristoff_core::ports::{SecretStore, SettingsStore};

/// Runs every `SecretStore` promise against fresh, empty stores from `make`.
pub fn secret_store_contract<S: SecretStore>(make: impl Fn() -> S) {
    reads_none_when_no_key_is_saved(&make());
    reads_back_the_saved_key_and_replaces_it(&make());
    deletes_the_key_and_deleting_twice_succeeds(&make());
}

fn reads_none_when_no_key_is_saved(store: &impl SecretStore) {
    assert_eq!(store.get_key().unwrap(), None);
}

fn reads_back_the_saved_key_and_replaces_it(store: &impl SecretStore) {
    store.set_key("sk-ant-first-0001").unwrap();
    store.set_key("sk-ant-second-0002").unwrap();

    assert_eq!(
        store.get_key().unwrap().as_deref(),
        Some("sk-ant-second-0002")
    );
    store.delete_key().unwrap();
}

fn deletes_the_key_and_deleting_twice_succeeds(store: &impl SecretStore) {
    store.set_key("sk-ant-gone-0003").unwrap();

    store.delete_key().unwrap();
    store.delete_key().unwrap();

    assert_eq!(store.get_key().unwrap(), None);
}

/// Runs every `SettingsStore` promise against fresh stores from `make`.
pub fn settings_store_contract<S: SettingsStore>(make: impl Fn() -> S) {
    reads_no_cap_when_never_set(&make());
    reads_back_the_latest_cap(&make());
}

fn reads_no_cap_when_never_set(store: &impl SettingsStore) {
    assert_eq!(store.daily_question_cap().unwrap(), None);
}

fn reads_back_the_latest_cap(store: &impl SettingsStore) {
    store.set_daily_question_cap(250).unwrap();
    store.set_daily_question_cap(40).unwrap();

    assert_eq!(store.daily_question_cap().unwrap(), Some(40));
}
