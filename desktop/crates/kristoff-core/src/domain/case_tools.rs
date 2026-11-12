//! The case tool rules (docs/desktop/phases/phase-7-omnicorpus.md, "Tools"): validating
//! `search_ph_cases` and `get_case` input against the shared bounds, and turning decisions into
//! citable search results titled "G.R. No. … · short title (year)". Pure.

use crate::domain::case::{CaseFilters, CaseHit, CaseView};
use crate::domain::model_request::{RawCaseSearchInput, RawGetCaseInput, SearchResult};
use crate::domain::tool_input::{bounded_text, limit, required_query, year};
use crate::limits::{TOOL_CASE_MAX, TOOL_FOCUS_MAX};

/// Sent as the tool result when a case search found nothing.
pub const NO_CASES_TEXT: &str = "No matching decisions were found in the indexed decisions.";

/// A validated case tool call, ready for the case index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseToolRun {
    /// `search_ph_cases`.
    Search {
        /// Trimmed search text.
        query: String,
        /// Narrowing.
        filters: CaseFilters,
        /// Most results (1–8).
        limit: u8,
    },
    /// `get_case`.
    Get {
        /// The decision: a case id or a G.R. number.
        case: String,
        /// What to look for in it.
        focus: Option<String>,
    },
}

fn case_year(value: Option<i64>, name: &str) -> Result<Option<u16>, String> {
    Ok(year(value, name)?.and_then(|y| u16::try_from(y).ok()))
}

/// Checks a `search_ph_cases` call against the shared bounds.
///
/// # Errors
/// A short message for an `is_error` tool result.
pub fn validate_case_search(raw: &RawCaseSearchInput) -> Result<CaseToolRun, String> {
    let query = required_query(raw.query.as_deref())?;
    let filters = CaseFilters {
        year_from: case_year(raw.year_from, "year_from")?,
        year_to: case_year(raw.year_to, "year_to")?,
    };
    let limit = u8::try_from(limit(raw.limit)?).unwrap_or(8);
    Ok(CaseToolRun::Search {
        query,
        filters,
        limit,
    })
}

/// Checks a `get_case` call against the shared bounds.
///
/// # Errors
/// A short message for an `is_error` tool result.
pub fn validate_get_case(raw: &RawGetCaseInput) -> Result<CaseToolRun, String> {
    let case = bounded_text(raw.case.as_deref(), "case", TOOL_CASE_MAX)?
        .ok_or_else(|| "case is required".to_owned())?;
    let focus = bounded_text(raw.focus.as_deref(), "focus", TOOL_FOCUS_MAX)?;
    Ok(CaseToolRun::Get { case, focus })
}

/// `G.R. No. 135981 · People v. Genosa (2004)`; without a G.R. number, `People v. Genosa (2004)`.
#[must_use]
pub fn case_title(hit: &CaseHit) -> String {
    let year = hit.date.get(..4).unwrap_or(&hit.date);
    let short = format!("{} ({year})", hit.title.trim());
    match &hit.gr_number {
        Some(gr) => format!("{} \u{b7} {short}", gr.trim()),
        None => short,
    }
}

/// One citable result for a decision with `text`; `None` without an `https` E-Library link (the
/// web opens sources only over https).
#[must_use]
pub fn case_result(hit: &CaseHit, text: &str) -> Option<SearchResult> {
    let source = hit
        .source_url
        .as_deref()
        .filter(|u| u.starts_with("https://"))?;
    Some(SearchResult {
        source: source.to_owned(),
        title: case_title(hit),
        text: text.to_owned(),
    })
}

/// The decision a `get_case` returned, as one result whose text is its passages.
#[must_use]
pub fn view_result(view: &CaseView) -> Option<SearchResult> {
    let text = if view.passages.is_empty() {
        view.hit.excerpt.clone()
    } else {
        view.passages.join("\n\n")
    };
    case_result(&view.hit, &text)
}
