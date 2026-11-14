//! `get_key_status`, `set_api_key`, `remove_api_key`, `get_settings` and `update_settings`
//! (Phase 8, docs/contract.md "Desktop IPC"; docs/desktop/phases/phase-8-lexph-byok.md §1). The key
//! goes in through `set_api_key` only; nothing returns more than its last 4 characters.

use super::docs::spawn_processing;
use super::{CommandResult, Shared, blocking};
use crate::composition::Composition;
use kristoff_contracts::envelopes::{KeyStatusResponse, SettingsResponse};
use std::sync::Arc;
use tauri::State;

/// The saved key's status.
///
/// # Errors
/// `INTERNAL` when the Keychain cannot be read.
pub fn key_status(app: &Composition) -> CommandResult<KeyStatusResponse> {
    Ok(KeyStatusResponse::from(&app.key_status()?))
}

/// Checks `key` (format, then live), saves it and makes it the key of every Claude call.
///
/// # Errors
/// `KEY_INVALID`, `UPSTREAM_UNAVAILABLE`, `INTERNAL`.
pub fn set_key(app: &Composition, key: &str) -> CommandResult<KeyStatusResponse> {
    Ok(KeyStatusResponse::from(&app.save_key(key)?))
}

/// Deletes the saved key.
///
/// # Errors
/// `INTERNAL` when the Keychain cannot be written.
pub fn remove_key(app: &Composition) -> CommandResult<KeyStatusResponse> {
    Ok(KeyStatusResponse::from(&app.remove_key()?))
}

/// The user settings.
///
/// # Errors
/// `INTERNAL` on a storage failure.
pub fn settings(app: &Composition) -> CommandResult<SettingsResponse> {
    Ok(SettingsResponse {
        daily_question_cap: app.daily_question_cap()?,
    })
}

/// Saves a new daily question limit.
///
/// # Errors
/// `VALIDATION_FAILED` outside 1..=10000; `INTERNAL` on a storage failure.
pub fn update(app: &Composition, daily_question_cap: u32) -> CommandResult<SettingsResponse> {
    Ok(SettingsResponse {
        daily_question_cap: app.set_daily_question_cap(daily_question_cap)?,
    })
}

/// `get_key_status {}`.
///
/// # Errors
/// Those of [`key_status`], as an `ErrorDto`.
#[tauri::command]
pub async fn get_key_status(app: State<'_, Shared>) -> CommandResult<KeyStatusResponse> {
    blocking(&app, key_status).await
}

/// `set_api_key {key}`. Docs left waiting for a key are processed once it is saved.
///
/// # Errors
/// Those of [`set_key`], as an `ErrorDto`.
#[tauri::command]
pub async fn set_api_key(app: State<'_, Shared>, key: String) -> CommandResult<KeyStatusResponse> {
    let shared = Arc::clone(app.inner());
    let status = blocking(&app, move |c| set_key(c, &key)).await?;
    match shared.docs_to_resume() {
        Ok(waiting) => spawn_processing(shared, waiting),
        Err(e) => shared.warn("doc.resume_failed", e.code.as_str()),
    }
    Ok(status)
}

/// `remove_api_key {}`.
///
/// # Errors
/// Those of [`remove_key`], as an `ErrorDto`.
#[tauri::command]
pub async fn remove_api_key(app: State<'_, Shared>) -> CommandResult<KeyStatusResponse> {
    blocking(&app, remove_key).await
}

/// `get_settings {}`.
///
/// # Errors
/// Those of [`settings`], as an `ErrorDto`.
#[tauri::command]
pub async fn get_settings(app: State<'_, Shared>) -> CommandResult<SettingsResponse> {
    blocking(&app, settings).await
}

/// `update_settings {dailyQuestionCap}`.
///
/// # Errors
/// Those of [`update`], as an `ErrorDto`.
#[tauri::command]
pub async fn update_settings(
    app: State<'_, Shared>,
    daily_question_cap: u32,
) -> CommandResult<SettingsResponse> {
    blocking(&app, move |c| update(c, daily_question_cap)).await
}
