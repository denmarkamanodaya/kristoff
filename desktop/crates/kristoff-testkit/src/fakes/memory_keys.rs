//! Fakes of the Phase 8 key ports: an in-memory `SecretStore`, a scripted `KeyVerifier`, and
//! `SettingsStore` on `MemoryStore`.

use super::memory_store::MemoryStore;
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::ports::{KeyVerifier, SecretStore, SettingsStore};
use std::sync::{Mutex, PoisonError};

/// Keeps the key in memory.
#[derive(Debug, Default)]
pub struct MemorySecretStore(Mutex<Option<String>>);

impl MemorySecretStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A store already holding `key`.
    #[must_use]
    pub fn holding(key: &str) -> Self {
        Self(Mutex::new(Some(key.to_owned())))
    }

    fn slot(&self) -> std::sync::MutexGuard<'_, Option<String>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl SecretStore for MemorySecretStore {
    fn get_key(&self) -> AppResult<Option<String>> {
        Ok(self.slot().clone())
    }

    fn set_key(&self, key: &str) -> AppResult<()> {
        *self.slot() = Some(key.to_owned());
        Ok(())
    }

    fn delete_key(&self) -> AppResult<()> {
        *self.slot() = None;
        Ok(())
    }
}

/// Accepts exactly one key; any other key fails with the configured code.
#[derive(Debug)]
pub struct FakeKeyVerifier {
    accepted: String,
    failure: ErrorCode,
    checked: Mutex<Vec<String>>,
}

impl FakeKeyVerifier {
    /// Accepts `key`; rejects every other key with `KEY_INVALID`.
    #[must_use]
    pub fn accepting(key: &str) -> Self {
        Self {
            accepted: key.to_owned(),
            failure: ErrorCode::KeyInvalid,
            checked: Mutex::new(Vec::new()),
        }
    }

    /// Fails every key with `UPSTREAM_UNAVAILABLE` (Anthropic unreachable).
    #[must_use]
    pub fn unreachable() -> Self {
        Self {
            accepted: String::new(),
            failure: ErrorCode::UpstreamUnavailable,
            checked: Mutex::new(Vec::new()),
        }
    }

    /// Every key passed to `verify`, in order.
    #[must_use]
    pub fn checked(&self) -> Vec<String> {
        self.checked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl KeyVerifier for FakeKeyVerifier {
    fn verify(&self, key: &str) -> AppResult<()> {
        self.checked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(key.to_owned());
        if !self.accepted.is_empty() && key == self.accepted {
            return Ok(());
        }
        Err(AppError::new(self.failure, "fake key check failed"))
    }
}

impl SettingsStore for MemoryStore {
    fn daily_question_cap(&self) -> AppResult<Option<u32>> {
        Ok(self.state().daily_question_cap)
    }

    fn set_daily_question_cap(&self, cap: u32) -> AppResult<()> {
        self.state().daily_question_cap = Some(cap);
        Ok(())
    }
}
