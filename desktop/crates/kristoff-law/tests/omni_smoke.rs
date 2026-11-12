//! Phase 7 smoke test: `OmniCorpus` codes and Supreme Court decisions in the built indexes.
//! Ignored by default (needs `data/phlaws.db`, `data/phcases.db` and the model). Fixture:
//! `contracts/fixtures/law-smoke-omnicorpus.json` (`KRISTOFF_OMNI_SMOKE`).

use std::error::Error;
use std::path::PathBuf;

use kristoff_core::domain::case::CaseFilters;
use kristoff_core::domain::law::LawFilters;
use kristoff_core::domain::law::reference::parse_references;
use kristoff_core::ports::{CaseSearch, LawSearch};
use kristoff_embed::{CandleEmbedder, DevicePreference};
use kristoff_law::{CaseIndex, LawIndex};

#[derive(serde::Deserialize)]
struct Fixture {
    codes: Vec<Code>,
    articles: Vec<ArticleCase>,
    cases: Vec<CaseLookup>,
}

#[derive(serde::Deserialize)]
struct Code {
    query: String,
    identifier: String,
    label: String,
    edition: String,
}

#[derive(serde::Deserialize)]
struct ArticleCase {
    law: String,
    article: String,
    identifier: String,
    label: String,
    starts_with: String,
}

#[derive(serde::Deserialize)]
struct CaseLookup {
    query: String,
    gr_number: String,
}

fn path_from_env(key: &str, default_rel: &str) -> PathBuf {
    std::env::var_os(key).map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(default_rel),
        PathBuf::from,
    )
}

fn embedder() -> Result<CandleEmbedder, Box<dyn Error>> {
    let model = path_from_env("KRISTOFF_MODEL_DIR", "../../data/model");
    Ok(CandleEmbedder::load(
        &model,
        DevicePreference::MetalThenCpu,
    )?)
}

fn fixture() -> Result<Fixture, Box<dyn Error>> {
    let path = path_from_env(
        "KRISTOFF_OMNI_SMOKE",
        "../../../contracts/fixtures/law-smoke-omnicorpus.json",
    );
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn law_index() -> Result<LawIndex, Box<dyn Error>> {
    let db = path_from_env("KRISTOFF_PHLAWS_DB", "../../data/phlaws.db");
    Ok(LawIndex::open(&db, Box::new(embedder()?))?)
}

#[test]
#[ignore = "needs data/phlaws.db and data/model"]
fn code_lookups_find_the_provision_with_its_edition_in_the_top_five() {
    let index = law_index().unwrap();
    let mut misses = Vec::new();

    for case in &fixture().unwrap().codes {
        let hits = index
            .search(&case.query, &LawFilters::default(), 5)
            .unwrap();
        let found = hits
            .iter()
            .position(|h| h.law.identifier == case.identifier && h.label == case.label);
        println!("{:?} rank {:?}", case.query, found.map(|i| i + 1));
        match found {
            Some(i) => assert_eq!(
                hits[i].edition.as_deref(),
                Some(case.edition.as_str()),
                "{}",
                case.query
            ),
            None => misses.push(case.query.clone()),
        }
    }

    assert!(misses.is_empty(), "missed: {misses:?}");
}

#[test]
#[ignore = "needs data/phlaws.db and data/model"]
fn get_article_reaches_code_provisions_by_their_names() {
    let index = law_index().unwrap();

    for case in &fixture().unwrap().articles {
        let law = parse_references(&case.law).laws[0];
        let view = index
            .get_article(law, &case.article)
            .unwrap()
            .unwrap_or_else(|| panic!("{} {}", case.law, case.article));
        assert_eq!(
            (view.hit.law.identifier.as_str(), view.hit.label.as_str()),
            (case.identifier.as_str(), case.label.as_str())
        );
        assert!(
            view.hit.text.starts_with(&case.starts_with),
            "{}: {}",
            case.label,
            &view.hit.text[..80.min(view.hit.text.len())]
        );
    }
}

#[test]
#[ignore = "needs data/phcases.db and data/model"]
fn case_lookups_find_the_decision_in_the_top_five_and_by_gr_number() {
    let db = path_from_env("KRISTOFF_PHCASES_DB", "../../data/phcases.db");
    let index = CaseIndex::open(&db, Box::new(embedder().unwrap())).unwrap();
    let mut misses = Vec::new();

    for case in &fixture().unwrap().cases {
        let hits = index
            .search(&case.query, &CaseFilters::default(), 5)
            .unwrap();
        let rank = hits
            .iter()
            .position(|h| h.gr_number.as_deref() == Some(case.gr_number.as_str()));
        println!("{:?} rank {:?}", case.query, rank.map(|i| i + 1));
        if rank.is_none() {
            misses.push(case.query.clone());
        }
        let view = index.get_case(&case.gr_number, None).unwrap().unwrap();
        assert_eq!(view.hit.gr_number.as_deref(), Some(case.gr_number.as_str()));
    }

    assert!(misses.is_empty(), "missed: {misses:?}");
}
