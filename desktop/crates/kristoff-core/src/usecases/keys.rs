//! The bring-your-own Claude key and the user settings (Phase 8,
//! docs/desktop/phases/phase-8-lexph-byok.md §1; docs/contract.md "Desktop IPC"): key status,
//! save (format check, live check, store), remove, and the daily question limit.

use crate::error::{AppError, AppResult, ErrorCode};
use crate::limits::{API_KEY_PREFIX, MAX_API_KEY, MAX_DAILY_QUESTION_CAP, MIN_DAILY_QUESTION_CAP};
use crate::ports::{KeyVerifier, SecretStore, SettingsStore};

/// Characters of the key the client may see.
const HINT_CHARS: usize = 4;

/// What the client may know about the saved key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyStatus {
    /// A key is saved.
    pub configured: bool,
    /// The key's last 4 characters, when one is saved.
    pub hint: Option<String>,
}

impl KeyStatus {
    /// The status of `key` (`None` = nothing saved). Never holds more than the last 4 characters.
    #[must_use]
    pub fn of(key: Option<&str>) -> Self {
        let hint = key.map(|k| {
            let chars: Vec<char> = k.chars().collect();
            chars[chars.len().saturating_sub(HINT_CHARS)..]
                .iter()
                .collect()
        });
        Self {
            configured: key.is_some(),
            hint,
        }
    }
}

/// The saved key's status.
///
/// # Errors
/// `INTERNAL` when the store cannot be read.
pub fn key_status(store: &dyn SecretStore) -> AppResult<KeyStatus> {
    Ok(KeyStatus::of(store.get_key()?.as_deref()))
}

/// A pasted key, trimmed, when it has the shape of a Claude API key.
///
/// # Errors
/// `KEY_INVALID` for an empty key, a missing `sk-ant-` prefix, inner whitespace or more than 256
/// bytes.
pub fn check_key_format(raw: &str) -> AppResult<&str> {
    let key = raw.trim();
    let well_formed = key.len() > API_KEY_PREFIX.len()
        && key.len() <= MAX_API_KEY
        && key.starts_with(API_KEY_PREFIX)
        && !key.chars().any(char::is_whitespace);
    if !well_formed {
        return Err(AppError::new(
            ErrorCode::KeyInvalid,
            "that does not look like a Claude API key",
        ));
    }
    Ok(key)
}

/// Checks `raw` (format, then live with Anthropic) and saves it.
///
/// # Errors
/// `KEY_INVALID` (bad format or rejected), `UPSTREAM_UNAVAILABLE` (the check could not be made),
/// `INTERNAL` (the store failed). Nothing is saved on any error.
pub fn save_key(
    store: &dyn SecretStore,
    verifier: &dyn KeyVerifier,
    raw: &str,
) -> AppResult<KeyStatus> {
    let key = check_key_format(raw)?;
    verifier.verify(key)?;
    store.set_key(key)?;
    Ok(KeyStatus::of(Some(key)))
}

/// Deletes the saved key.
///
/// # Errors
/// `INTERNAL` when the store cannot be written.
pub fn remove_key(store: &dyn SecretStore) -> AppResult<KeyStatus> {
    store.delete_key()?;
    Ok(KeyStatus::of(None))
}

/// The daily question limit in force: the user's, else `default_cap`.
///
/// # Errors
/// `INTERNAL` on a storage failure (fail closed: no guessed limit).
pub fn daily_question_cap(settings: &dyn SettingsStore, default_cap: u32) -> AppResult<u32> {
    Ok(settings.daily_question_cap()?.unwrap_or(default_cap))
}

/// Saves a new daily question limit.
///
/// # Errors
/// `VALIDATION_FAILED` outside 1..=10000; `INTERNAL` on a storage failure.
pub fn update_daily_question_cap(settings: &dyn SettingsStore, cap: u32) -> AppResult<u32> {
    if !(MIN_DAILY_QUESTION_CAP..=MAX_DAILY_QUESTION_CAP).contains(&cap) {
        return Err(AppError::new(
            ErrorCode::ValidationFailed,
            format!(
                "the daily question limit must be from {MIN_DAILY_QUESTION_CAP} to \
                 {MAX_DAILY_QUESTION_CAP}"
            ),
        ));
    }
    settings.set_daily_question_cap(cap)?;
    Ok(cap)
}
