//! Get config and get usage (ported from `get-config` and `get-usage`).

use crate::error::AppResult;
use crate::model::UsageReport;
use crate::ports::{Clock, UsageMeter};
use crate::util::time::utc_day;

/// One selectable model: its API id and the label the web shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelOption {
    /// API model id.
    pub id: String,
    /// Shown label.
    pub label: String,
}

/// The configuration values the composition root passes in.
#[derive(Debug, Clone, Copy)]
pub struct ConfigSource<'a> {
    /// Allowed model ids, first = default.
    pub model_ids: &'a [&'a str],
    /// Labels by model id; an id without a label shows the id.
    pub labels: &'a [(&'a str, &'a str)],
    /// Most attachments per chat.
    pub max_attached_docs: usize,
    /// Questions per UTC day.
    pub daily_question_cap: u32,
}

/// The settings the web shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientConfig {
    /// The default model id (the first allowed one; empty when none is configured).
    pub model: String,
    /// The allowed models in allowlist order.
    pub models: Vec<ModelOption>,
    /// Most attachments per chat.
    pub max_attached_docs: usize,
    /// Questions per UTC day.
    pub daily_question_cap: u32,
}

/// The client-visible settings.
#[must_use]
pub fn get_config(source: &ConfigSource<'_>) -> ClientConfig {
    let label = |id: &str| -> String {
        let found = source.labels.iter().find(|(k, _)| *k == id);
        found.map_or(id, |(_, l)| *l).to_owned()
    };
    ClientConfig {
        model: source
            .model_ids
            .first()
            .map(|id| (*id).to_owned())
            .unwrap_or_default(),
        models: source
            .model_ids
            .iter()
            .map(|id| ModelOption {
                id: (*id).to_owned(),
                label: label(id),
            })
            .collect(),
        max_attached_docs: source.max_attached_docs,
        daily_question_cap: source.daily_question_cap,
    }
}

/// Today's (UTC) and all-time token counters.
///
/// # Errors
/// Meter failures.
pub fn get_usage(meter: &dyn UsageMeter, clock: &dyn Clock) -> AppResult<UsageReport> {
    meter.read_usage(&utc_day(clock.now_ms()))
}
