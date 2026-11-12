//! `CaseSearch` contract. It is parameterised by one decision the index holds ([`KnownCase`]), so
//! the real index runs it against a decision in the built `phcases.db` and the fake against
//! [`SAMPLE_CASE`].

use kristoff_core::domain::case::{CaseFilters, CaseHit};
use kristoff_core::ports::CaseSearch;

/// A decision the index under test holds, and how to find it.
#[derive(Debug, Clone, Copy)]
pub struct KnownCase {
    /// Its G.R. number as stored, e.g. `G.R. No. 135981`.
    pub gr_number: &'static str,
    /// A query that must rank it within the first 8 results.
    pub query: &'static str,
    /// Its promulgation year.
    pub year: u16,
    /// A word that occurs in at least one of its passages.
    pub focus: &'static str,
}

/// The decision [`crate::fakes::MemoryCaseSearch::sample`] holds first.
pub const SAMPLE_CASE: KnownCase = KnownCase {
    gr_number: "G.R. No. 900001",
    query: "battered woman syndrome defense acquitted",
    year: 2004,
    focus: "reconciliation",
};

fn has(hits: &[CaseHit], known: &KnownCase) -> bool {
    hits.iter()
        .any(|h| h.gr_number.as_deref() == Some(known.gr_number))
}

/// Runs every `CaseSearch` promise against `search`, which holds `known`.
///
/// # Panics
/// On the first broken promise.
pub fn case_search_contract(search: &impl CaseSearch, known: &KnownCase) {
    search.warm().unwrap();
    assert!(search.case_count().unwrap() >= 1);
    finds_the_known_case_within_the_limit(search, known);
    year_filters_hold(search, known);
    get_case_returns_the_decision_with_at_most_three_passages(search, known);
}

fn finds_the_known_case_within_the_limit(search: &impl CaseSearch, known: &KnownCase) {
    let hits = search
        .search(known.query, &CaseFilters::default(), 8)
        .unwrap();
    assert!(hits.len() <= 8);
    assert!(has(&hits, known), "{} not in {hits:?}", known.gr_number);
    assert!(
        search
            .search(known.query, &CaseFilters::default(), 1)
            .unwrap()
            .len()
            <= 1
    );
    let mut ids: Vec<&str> = hits.iter().map(|h| h.case_id.as_str()).collect();
    ids.dedup();
    assert_eq!(ids.len(), hits.len(), "one hit per decision");
    for h in &hits {
        assert!(
            h.source_url
                .as_deref()
                .is_none_or(|u| u.starts_with("https://")),
            "{:?}",
            h.source_url
        );
        assert!(!h.title.is_empty() && h.date.len() >= 4);
    }
}

fn year_filters_hold(search: &impl CaseSearch, known: &KnownCase) {
    let only = CaseFilters {
        year_from: Some(known.year),
        year_to: Some(known.year),
    };
    let hits = search.search(known.query, &only, 8).unwrap();
    assert!(has(&hits, known));
    assert!(
        hits.iter()
            .all(|h| h.date.starts_with(&known.year.to_string())),
        "{hits:?}"
    );
    let after = CaseFilters {
        year_from: Some(known.year + 1),
        year_to: None,
    };
    assert!(!has(&search.search(known.query, &after, 8).unwrap(), known));
}

fn get_case_returns_the_decision_with_at_most_three_passages(
    search: &impl CaseSearch,
    known: &KnownCase,
) {
    let digits: String = known
        .gr_number
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    for name in [known.gr_number, digits.as_str()] {
        let view = search
            .get_case(name, None)
            .unwrap()
            .unwrap_or_else(|| panic!("{name} not found"));
        assert_eq!(view.hit.gr_number.as_deref(), Some(known.gr_number));
        assert!(!view.passages.is_empty() && view.passages.len() <= 3);
    }
    let focused = search
        .get_case(known.gr_number, Some(known.focus))
        .unwrap()
        .unwrap();
    assert!(focused.passages.len() <= 3);
    let focus = known.focus.to_lowercase();
    assert!(
        focused
            .passages
            .iter()
            .any(|p| p.to_lowercase().contains(&focus)),
        "{focused:?}"
    );
    assert_eq!(search.get_case("G.R. No. 1", None).unwrap(), None);
}
