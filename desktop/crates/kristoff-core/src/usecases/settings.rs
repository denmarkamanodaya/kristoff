//! Get config and get usage (ported from `get-config` and `get-usage`).

use crate::error::AppResult;
use crate::model::UsageReport;
use crate::ports::{CaseSearch, Clock, LawSearch, UsageMeter};
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

/// What the desktop offers beyond the web contract (Phase 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Features {
    /// Statute search is available (the law index is installed).
    pub statutes: bool,
    /// Laws in the index, when it is available and says.
    pub law_count: Option<u64>,
    /// Nothing is left to load: both indexes have warmed, or there is none to wait for.
    pub law_ready: bool,
    /// Case search is available (the decisions index is installed; Phase 7).
    pub cases: bool,
    /// Decisions in the index, when it is available and can count them.
    pub case_count: Option<u64>,
}

/// The features given the law and case indexes (`None` when the build has one or both missing) and
/// whether the warm-up of both has finished. With no index at all, `law_ready` is true at once, so a
/// client never waits forever.
#[must_use]
pub fn features(
    law: Option<&dyn LawSearch>,
    cases: Option<&dyn CaseSearch>,
    warmed: bool,
) -> Features {
    Features {
        statutes: law.is_some(),
        // Recovery: the counts are display-only; an index that cannot read them still searches.
        law_count: law.and_then(|l| l.law_count().ok().flatten()),
        law_ready: (law.is_none() && cases.is_none()) || warmed,
        cases: cases.is_some(),
        case_count: cases.and_then(|c| c.case_count().ok()),
    }
}

/// Today's (UTC) and all-time token counters.
///
/// # Errors
/// Meter failures.
pub fn get_usage(meter: &dyn UsageMeter, clock: &dyn Clock) -> AppResult<UsageReport> {
    meter.read_usage(&utc_day(clock.now_ms()))
}
