//! Bounds checks shared by every tool's input (the one rule for strings, years and limits). Pure.

use crate::limits::{TOOL_QUERY_MAX, TOOL_RESULTS_MAX, TOOL_YEAR_MAX, TOOL_YEAR_MIN};

/// A trimmed string of 1 to `max` characters; `Ok(None)` when absent.
///
/// # Errors
/// A short message naming `name` and the bounds.
pub(crate) fn bounded_text(
    value: Option<&str>,
    name: &str,
    max: usize,
) -> Result<Option<String>, String> {
    let Some(text) = value.map(str::trim) else {
        return Ok(None);
    };
    if text.is_empty() || text.chars().count() > max {
        return Err(format!("{name} must be 1 to {max} characters"));
    }
    Ok(Some(text.to_owned()))
}

/// The required `query` of a search tool, trimmed and bounded.
///
/// # Errors
/// A short message when it is absent or out of bounds.
pub(crate) fn required_query(value: Option<&str>) -> Result<String, String> {
    bounded_text(value, "query", TOOL_QUERY_MAX)?.ok_or_else(|| "query is required".to_owned())
}

/// A year within `TOOL_YEAR_MIN..=TOOL_YEAR_MAX`; `Ok(None)` when absent.
///
/// # Errors
/// A short message naming `name` and the bounds.
pub(crate) fn year(value: Option<i64>, name: &str) -> Result<Option<u32>, String> {
    let Some(y) = value else { return Ok(None) };
    u32::try_from(y)
        .ok()
        .filter(|y| (TOOL_YEAR_MIN..=TOOL_YEAR_MAX).contains(y))
        .map(Some)
        .ok_or_else(|| format!("{name} must be between {TOOL_YEAR_MIN} and {TOOL_YEAR_MAX}"))
}

/// A result count of 1 to `TOOL_RESULTS_MAX`; the maximum when absent.
///
/// # Errors
/// A short message with the bounds.
pub(crate) fn limit(value: Option<i64>) -> Result<usize, String> {
    let max = i64::try_from(TOOL_RESULTS_MAX).unwrap_or(8);
    match value {
        None => Ok(TOOL_RESULTS_MAX),
        Some(n) if (1..=max).contains(&n) => Ok(usize::try_from(n).unwrap_or(TOOL_RESULTS_MAX)),
        Some(_) => Err(format!("limit must be 1 to {TOOL_RESULTS_MAX}")),
    }
}
