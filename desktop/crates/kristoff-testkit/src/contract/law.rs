//! `LawSearch` contract. The real index runs it against the built `phlaws.db`, so every promise
//! names provisions that exist there (RA 9262 Secs. 4–6, Constitution Art. III Sec. 2) and that
//! `MemoryLawSearch::sample` also holds.

use kristoff_core::domain::law::{LawFilters, LawHit, LawRef};
use kristoff_core::ports::LawSearch;

const VAWC: &str = "Republic Act No. 9262";

fn first(hits: &[LawHit]) -> (&str, &str) {
    let hit = hits.first().expect("at least one hit");
    (hit.law.identifier.as_str(), hit.label.as_str())
}

/// Runs every `LawSearch` promise against `search`, which holds the provisions named above.
///
/// # Panics
/// On the first broken promise.
pub fn law_search_contract(search: &impl LawSearch) {
    search.warm().unwrap();
    a_cited_provision_ranks_first(search);
    limits_and_filters_hold(search);
    get_article_returns_the_provision_and_its_neighbours(search);
}

fn a_cited_provision_ranks_first(search: &impl LawSearch) {
    let hits = search
        .search("RA 9262 Sec. 5", &LawFilters::default(), 5)
        .unwrap();
    assert!(hits.len() <= 5);
    assert_eq!(first(&hits), (VAWC, "SEC. 5"));
    let hits = search
        .search("Article III Sec. 2 Constitution", &LawFilters::default(), 5)
        .unwrap();
    assert_eq!(first(&hits), ("1987 Constitution", "ARTICLE III SECTION 2"));
}

fn limits_and_filters_hold(search: &impl LawSearch) {
    let query = "violence against women and their children";
    assert_eq!(
        search
            .search(query, &LawFilters::default(), 1)
            .unwrap()
            .len(),
        1
    );
    let by_number = LawFilters {
        law_number: Some(9262),
        ..LawFilters::default()
    };
    let hits = search.search(query, &by_number, 8).unwrap();
    assert_ne!(hits.len(), 0);
    assert!(hits.iter().all(|h| h.law.number == Some(9262)));
    let by_year = LawFilters {
        year_from: Some(2004),
        year_to: Some(2004),
        law_number: None,
    };
    let hits = search.search(query, &by_year, 8).unwrap();
    assert_ne!(hits.len(), 0);
    assert!(hits.iter().all(|h| h.law.year == Some(2004)));
}

fn get_article_returns_the_provision_and_its_neighbours(search: &impl LawSearch) {
    let view = search
        .get_article(LawRef::RepublicAct(9262), "Sec. 5")
        .unwrap()
        .unwrap();
    assert_eq!(
        (view.hit.law.identifier.as_str(), view.hit.label.as_str()),
        (VAWC, "SEC. 5")
    );
    assert_eq!(view.hit.part, 0);
    assert!(view.hit.text.starts_with("SEC. 5. Acts of Violence"));
    assert_eq!(view.previous.unwrap().label, "SEC. 4");
    assert_eq!(view.next.unwrap().label, "SEC. 6");
    assert!(
        search
            .get_article(LawRef::RepublicAct(9262), "Sec. 999")
            .unwrap()
            .is_none()
    );
    assert!(
        search
            .get_article(LawRef::RepublicAct(99_999), "Sec. 1")
            .unwrap()
            .is_none()
    );
}
