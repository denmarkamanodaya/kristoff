//! `verify_otp` (the only ungated command), `get_config` and `get_usage`.

use super::{CommandResult, Shared, blocking, unlocked};
use crate::composition::Composition;
use kristoff_contracts::config::LOCAL_TOKEN;
use kristoff_contracts::envelopes::{ConfigResponse, SessionResponse, UsageResponse};
use kristoff_contracts::requests;
use kristoff_contracts::shapes::{ModelOptionDto, UsageDto};
use kristoff_core::usecases::{auth, settings};
use kristoff_core::util::time::iso_instant;
use tauri::State;

/// Checks a TOTP code and opens the gate until the unlock ends.
///
/// # Errors
/// `VALIDATION_FAILED`, `OTP_INVALID`, `OTP_REPLAYED`, `OTP_LOCKED` (with `retryAfter`), `INTERNAL`.
pub fn verify(app: &Composition, code: &str) -> CommandResult<SessionResponse> {
    let unlock = auth::verify_otp(&app.auth(), requests::otp_code(code)?)?;
    app.gate.unlock(unlock.expires_at_ms);
    let expires_at = iso_instant(unlock.expires_at_ms);
    Ok(SessionResponse {
        token: LOCAL_TOKEN.to_owned(),
        expires_at,
    })
}

/// The client-visible settings.
///
/// # Errors
/// `UNAUTHENTICATED` while locked.
pub fn config(app: &Composition) -> CommandResult<ConfigResponse> {
    unlocked(app)?;
    let c = settings::get_config(&app.config());
    let models = c.models.into_iter().map(|m| ModelOptionDto {
        id: m.id,
        label: m.label,
    });
    Ok(ConfigResponse {
        model: c.model,
        models: models.collect(),
        max_attached_docs: c.max_attached_docs,
        daily_question_cap: c.daily_question_cap,
    })
}

/// Today's and all-time token counters.
///
/// # Errors
/// `UNAUTHENTICATED` while locked; `INTERNAL` on a storage failure.
pub fn usage(app: &Composition) -> CommandResult<UsageResponse> {
    unlocked(app)?;
    let (meter, clock) = app.usage();
    let report = settings::get_usage(meter, clock)?;
    Ok(UsageResponse {
        today: UsageDto::from(&report.today),
        total: UsageDto::from(&report.total),
    })
}

/// `verify_otp {code}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn verify_otp(app: State<'_, Shared>, code: String) -> CommandResult<SessionResponse> {
    blocking(&app, move |c| verify(c, &code)).await
}

/// `get_config`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn get_config(app: State<'_, Shared>) -> CommandResult<ConfigResponse> {
    blocking(&app, config).await
}

/// `get_usage`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn get_usage(app: State<'_, Shared>) -> CommandResult<UsageResponse> {
    blocking(&app, usage).await
}
