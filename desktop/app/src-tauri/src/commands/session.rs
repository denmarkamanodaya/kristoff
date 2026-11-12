//! `get_config` and `get_usage`.

use super::{CommandResult, Shared, blocking};
use crate::composition::Composition;
use kristoff_contracts::envelopes::{ConfigResponse, FeaturesDto, UsageResponse};
use kristoff_contracts::shapes::{ModelOptionDto, UsageDto};
use kristoff_core::usecases::settings;
use tauri::State;

/// The client-visible settings.
///
/// # Errors
/// None today; the `Result` keeps every command's shape.
pub fn config(app: &Composition) -> CommandResult<ConfigResponse> {
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
        features: {
            let f = settings::features(app.law_search(), app.case_search(), app.law_warmed());
            FeaturesDto {
                statutes: f.statutes,
                law_count: f.law_count,
                law_ready: f.law_ready,
                cases: f.cases,
                case_count: f.case_count,
            }
        },
    })
}

/// Today's and all-time token counters.
///
/// # Errors
/// `INTERNAL` on a storage failure.
pub fn usage(app: &Composition) -> CommandResult<UsageResponse> {
    let (meter, clock) = app.usage();
    let report = settings::get_usage(meter, clock)?;
    Ok(UsageResponse {
        today: UsageDto::from(&report.today),
        total: UsageDto::from(&report.total),
    })
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
