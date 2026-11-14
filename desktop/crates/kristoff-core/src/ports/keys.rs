//! Ports of the bring-your-own Claude key and the user settings (Phase 8,
//! docs/desktop/phases/phase-8-lexph-byok.md §1): where the key is kept, how a candidate key is
//! checked, and where the daily question limit is kept.

use crate::error::AppResult;

/// Keeps the one Claude API key (the macOS Keychain in production). The key never goes to logs.
pub trait SecretStore: Send + Sync {
    /// The saved key, or `None` when none is saved.
    ///
    /// # Errors
    /// `INTERNAL` when the store cannot be read.
    fn get_key(&self) -> AppResult<Option<String>>;
    /// Saves `key`, replacing any earlier one.
    ///
    /// # Errors
    /// `INTERNAL` when the store cannot be written.
    fn set_key(&self, key: &str) -> AppResult<()>;
    /// Deletes the saved key; deleting when none is saved succeeds.
    ///
    /// # Errors
    /// `INTERNAL` when the store cannot be written.
    fn delete_key(&self) -> AppResult<()>;
}

/// Checks a candidate key with Anthropic before it is saved.
pub trait KeyVerifier: Send + Sync {
    /// Succeeds when Anthropic accepts `key`.
    ///
    /// # Errors
    /// `KEY_INVALID` when Anthropic rejects it (401 or 403); `UPSTREAM_UNAVAILABLE` when the
    /// check cannot be made (network, timeout, 5xx).
    fn verify(&self, key: &str) -> AppResult<()>;
}

/// Keeps the user settings on this Mac.
pub trait SettingsStore: Send + Sync {
    /// The saved daily question limit, or `None` when the user never changed it.
    ///
    /// # Errors
    /// `INTERNAL` on a storage failure.
    fn daily_question_cap(&self) -> AppResult<Option<u32>>;
    /// Saves the daily question limit (already range-checked by the use case).
    ///
    /// # Errors
    /// `INTERNAL` on a storage failure.
    fn set_daily_question_cap(&self, cap: u32) -> AppResult<()>;
}
