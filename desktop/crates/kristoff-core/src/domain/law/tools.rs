//! The statute tool rules (docs/desktop/phases/phase-4-statute-tools.md §1): validating the
//! model's tool input against the bounds the tool schema advertises, and turning law passages
//! into citable search results. Pure.

use super::reference::parse_references;
use super::{ArticleView, LawFilters, LawHit, LawRef};
use crate::domain::model_request::{RawArticleInput, RawSearchInput, SearchResult, ToolInput};
use crate::limits::{
    TOOL_ARTICLE_MAX, TOOL_LAW_NUMBER_MAX, TOOL_QUERY_MAX, TOOL_RESULTS_MAX, TOOL_YEAR_MAX,
    TOOL_YEAR_MIN,
};

/// Sent as the tool result when a search found nothing.
pub const NO_RESULTS_TEXT: &str = "No matching provisions were found in the indexed laws.";

/// A validated tool call, ready for the law index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LawToolRun {
    /// `search_ph_laws`.
    Search {
        /// Trimmed search text.
        query: String,
        /// Narrowing.
        filters: LawFilters,
        /// Most results (1–8).
        limit: usize,
    },
    /// `get_law_article`.
    Article {
        /// The law.
        law: LawRef,
        /// The provision as the model wrote it.
        article: String,
    },
}

/// Checks a tool call against the schema bounds.
///
/// # Errors
/// A short message for an `is_error` tool result.
pub fn validate(input: &ToolInput) -> Result<LawToolRun, String> {
    match input {
        ToolInput::SearchLaws(raw) => validate_search(raw),
        ToolInput::GetLawArticle(raw) => validate_article(raw),
        ToolInput::Malformed(name) => Err(format!("the input of {name} does not match its schema")),
        ToolInput::Unknown(name) => Err(format!("there is no tool named {name}")),
    }
}

fn bounded_text(value: Option<&str>, name: &str, max: usize) -> Result<Option<String>, String> {
    let Some(text) = value.map(str::trim) else {
        return Ok(None);
    };
    if text.is_empty() || text.chars().count() > max {
        return Err(format!("{name} must be 1 to {max} characters"));
    }
    Ok(Some(text.to_owned()))
}

fn year(value: Option<i64>, name: &str) -> Result<Option<u32>, String> {
    let Some(y) = value else { return Ok(None) };
    u32::try_from(y)
        .ok()
        .filter(|y| (TOOL_YEAR_MIN..=TOOL_YEAR_MAX).contains(y))
        .map(Some)
        .ok_or_else(|| format!("{name} must be between {TOOL_YEAR_MIN} and {TOOL_YEAR_MAX}"))
}

fn law_ref(text: &str) -> Result<LawRef, String> {
    parse_references(text).laws.first().copied().ok_or_else(|| {
        "law_number must name a law, e.g. \"RA 9262\" or \"Constitution\"".to_owned()
    })
}

fn validate_search(raw: &RawSearchInput) -> Result<LawToolRun, String> {
    let query = bounded_text(raw.query.as_deref(), "query", TOOL_QUERY_MAX)?
        .ok_or_else(|| "query is required".to_owned())?;
    let law = bounded_text(raw.law_number.as_deref(), "law_number", TOOL_LAW_NUMBER_MAX)?;
    let law = law.as_deref().map(law_ref).transpose()?;
    let max = i64::try_from(TOOL_RESULTS_MAX).unwrap_or(8);
    let limit = match raw.limit {
        None => TOOL_RESULTS_MAX,
        Some(n) if (1..=max).contains(&n) => usize::try_from(n).unwrap_or(TOOL_RESULTS_MAX),
        Some(_) => return Err(format!("limit must be 1 to {TOOL_RESULTS_MAX}")),
    };
    let filters = LawFilters {
        year_from: year(raw.year_from, "year_from")?,
        year_to: year(raw.year_to, "year_to")?,
        law_number: match law {
            Some(LawRef::RepublicAct(n)) => Some(n),
            Some(LawRef::Constitution) | None => None,
        },
    };
    Ok(LawToolRun::Search {
        query,
        filters,
        limit,
    })
}

fn validate_article(raw: &RawArticleInput) -> Result<LawToolRun, String> {
    let law = bounded_text(raw.law_number.as_deref(), "law_number", TOOL_LAW_NUMBER_MAX)?
        .ok_or_else(|| "law_number is required".to_owned())?;
    let article = bounded_text(raw.article.as_deref(), "article", TOOL_ARTICLE_MAX)?
        .ok_or_else(|| "article is required".to_owned())?;
    Ok(LawToolRun::Article {
        law: law_ref(&law)?,
        article,
    })
}

/// The status line for a law's `status` (the dataset's `law_status`); `None` when in force.
/// Unknown values show no line (only `current` occurs in the 2026-11-12 snapshot).
#[must_use]
pub fn status_label(status: &str) -> Option<&'static str> {
    let normalised = status.trim().to_lowercase().replace(['_', '-'], " ");
    match normalised.as_str() {
        "repealed" => Some("Repealed"),
        "amended" => Some("Amended"),
        "partially repealed" => Some("Partially repealed"),
        "declared unconstitutional" | "unconstitutional" => Some("Declared unconstitutional"),
        _ => None,
    }
}

/// One citable result for a passage; `None` when its source is not an `https` link (the web opens
/// sources only over https, so such a passage is left out rather than cited without a link).
#[must_use]
pub fn search_result(hit: &LawHit) -> Option<SearchResult> {
    if !hit.source_url.starts_with("https://") {
        return None;
    }
    let status = status_label(&hit.status).or(hit.truncated.then_some("Text may be cut off"));
    let text = match status {
        Some(label) => format!("[{label}] {}", hit.text),
        None => hit.text.clone(),
    };
    Some(SearchResult {
        source: hit.source_url.clone(),
        title: format!("{} \u{b7} {}", hit.law.identifier, hit.label),
        text,
    })
}

/// The requested provision first, then the provisions before and after it.
#[must_use]
pub fn article_results(view: &ArticleView) -> Vec<SearchResult> {
    [Some(&view.hit), view.previous.as_ref(), view.next.as_ref()]
        .into_iter()
        .flatten()
        .filter_map(search_result)
        .collect()
}
