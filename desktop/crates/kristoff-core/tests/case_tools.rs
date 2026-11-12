//! The case tool rules (phase-7 "Tools"): input bounds, the display title, https-only sources and
//! the `get_case` passages.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::domain::case::{CaseFilters, CaseHit, CaseView};
use kristoff_core::domain::case_tools::{
    CaseToolRun, case_result, case_title, validate_case_search, validate_get_case, view_result,
};
use kristoff_core::domain::model_request::{RawCaseSearchInput, RawGetCaseInput};
use kristoff_core::limits::{TOOL_CASE_MAX, TOOL_FOCUS_MAX};

fn hit(gr: Option<&str>, url: Option<&str>) -> CaseHit {
    CaseHit {
        case_id: "d-1".into(),
        gr_number: gr.map(Into::into),
        title: " People v. Genosa ".into(),
        date: "2004-01-15".into(),
        ponente: None,
        excerpt: "excerpt".into(),
        source_url: url.map(Into::into),
    }
}

const URL: &str = "https://elibrary.judiciary.gov.ph/x";

#[test]
fn the_title_leads_with_the_gr_number_and_ends_with_the_year() {
    assert_eq!(
        case_title(&hit(Some("G.R. No. 135981"), None)),
        "G.R. No. 135981 \u{b7} People v. Genosa (2004)"
    );
    assert_eq!(case_title(&hit(None, None)), "People v. Genosa (2004)");
}

#[test]
fn a_decision_without_an_https_link_is_never_sent() {
    assert!(case_result(&hit(None, Some("http://elibrary.judiciary.gov.ph/x")), "t").is_none());
    assert!(case_result(&hit(None, None), "t").is_none());
    assert_eq!(case_result(&hit(None, Some(URL)), "t").unwrap().source, URL);
}

#[test]
fn a_viewed_decision_sends_its_passages_or_else_its_excerpt() {
    let view = CaseView {
        hit: hit(None, Some(URL)),
        passages: vec!["a".into(), "b".into()],
    };
    let bare = CaseView {
        hit: hit(None, Some(URL)),
        passages: vec![],
    };

    assert_eq!(view_result(&view).unwrap().text, "a\n\nb");
    assert_eq!(view_result(&bare).unwrap().text, "excerpt");
}

#[test]
fn a_case_search_is_trimmed_bounded_and_defaults_to_the_max_limit() {
    let raw = RawCaseSearchInput {
        query: Some("  rape ".into()),
        year_from: Some(2000),
        ..Default::default()
    };

    let run = validate_case_search(&raw).unwrap();

    let filters = CaseFilters {
        year_from: Some(2000),
        year_to: None,
    };
    assert_eq!(
        run,
        CaseToolRun::Search {
            query: "rape".into(),
            filters,
            limit: 8
        }
    );
}

#[test]
fn bad_case_search_input_is_refused_with_a_reason() {
    let no_query = RawCaseSearchInput::default();
    let bad_year = RawCaseSearchInput {
        query: Some("q".into()),
        year_to: Some(1800),
        ..Default::default()
    };
    let bad_limit = RawCaseSearchInput {
        query: Some("q".into()),
        limit: Some(9),
        ..Default::default()
    };

    assert!(
        validate_case_search(&no_query)
            .unwrap_err()
            .contains("query")
    );
    assert!(
        validate_case_search(&bad_year)
            .unwrap_err()
            .contains("year_to")
    );
    assert!(
        validate_case_search(&bad_limit)
            .unwrap_err()
            .contains("limit")
    );
}

#[test]
fn get_case_needs_a_bounded_case_and_bounds_its_focus() {
    let long_case = RawGetCaseInput {
        case: Some("9".repeat(TOOL_CASE_MAX + 1)),
        focus: None,
    };
    let long_focus = RawGetCaseInput {
        case: Some("G.R. No. 1".into()),
        focus: Some("f".repeat(TOOL_FOCUS_MAX + 1)),
    };

    assert!(
        validate_get_case(&RawGetCaseInput::default())
            .unwrap_err()
            .contains("case")
    );
    assert!(validate_get_case(&long_case).unwrap_err().contains("case"));
    assert!(
        validate_get_case(&long_focus)
            .unwrap_err()
            .contains("focus")
    );
}
