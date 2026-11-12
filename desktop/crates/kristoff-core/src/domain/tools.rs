//! One validation entry for every tool call: statute tools (`domain::law::tools`) and case tools
//! (`domain::case_tools`). Pure.

use crate::domain::case_tools::{CaseToolRun, validate_case_search, validate_get_case};
use crate::domain::law::tools::{LawToolRun, validate as validate_law};
use crate::domain::model_request::ToolInput;

/// A validated tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolRun {
    /// A statute tool call.
    Law(LawToolRun),
    /// A case tool call.
    Case(CaseToolRun),
}

/// Checks any tool call against its schema bounds.
///
/// # Errors
/// A short message for an `is_error` tool result.
pub fn validate(input: &ToolInput) -> Result<ToolRun, String> {
    match input {
        ToolInput::SearchCases(raw) => validate_case_search(raw).map(ToolRun::Case),
        ToolInput::GetCase(raw) => validate_get_case(raw).map(ToolRun::Case),
        other => validate_law(other).map(ToolRun::Law),
    }
}
