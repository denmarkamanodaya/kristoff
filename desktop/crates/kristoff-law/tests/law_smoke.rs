//! Phase 3 §3 smoke test against a built `phlaws.db`. Ignored by default: it needs the db
//! (`kristoff-ingest`) and the model (`scripts/fetch-model.sh`).
//! Paths: `KRISTOFF_PHLAWS_DB` (default `data/phlaws.db`), `KRISTOFF_MODEL_DIR` (default
//! `data/model`), `KRISTOFF_LAW_SMOKE` (default `contracts/fixtures/law-smoke.json`).

use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use kristoff_core::domain::law::{LawFilters, LawHit, LawRef};
use kristoff_core::ports::LawSearch;
use kristoff_embed::{CandleEmbedder, DevicePreference};
use kristoff_law::LawIndex;

#[derive(serde::Deserialize)]
struct Fixture {
    smoke: Vec<Smoke>,
    taglish: Vec<Taglish>,
}

#[derive(serde::Deserialize)]
struct Smoke {
    query: String,
    identifier: String,
    label: String,
}

#[derive(serde::Deserialize)]
struct Accept {
    identifier: String,
    label: String,
}

#[derive(serde::Deserialize)]
struct Taglish {
    query: String,
    accept: Vec<Accept>,
}

fn path_from_env(key: &str, default_rel: &str) -> PathBuf {
    std::env::var_os(key).map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(default_rel),
        PathBuf::from,
    )
}

fn open_index() -> Result<LawIndex, Box<dyn Error>> {
    let model = path_from_env("KRISTOFF_MODEL_DIR", "../../data/model");
    let db = path_from_env("KRISTOFF_PHLAWS_DB", "../../data/phlaws.db");
    let embedder = CandleEmbedder::load(&model, DevicePreference::MetalThenCpu)?;
    let index = LawIndex::open(&db, Box::new(embedder))?;
    index.warm()?;
    Ok(index)
}

fn load_fixture() -> Result<Fixture, Box<dyn Error>> {
    let path = path_from_env(
        "KRISTOFF_LAW_SMOKE",
        "../../../contracts/fixtures/law-smoke.json",
    );
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn rank_of(hits: &[LawHit], identifier: &str, label: &str) -> Option<usize> {
    hits.iter()
        .position(|h| h.law.identifier == identifier && h.label == label)
        .map(|i| i + 1)
}

fn show(hits: &[LawHit]) -> String {
    hits.iter()
        .map(|h| format!("{} · {}", h.law.identifier, h.label))
        .collect::<Vec<_>>()
        .join(" | ")
}

#[test]
#[ignore = "needs data/phlaws.db and data/model"]
fn twenty_lookups_find_the_expected_article_in_the_top_five() {
    let index = open_index().unwrap();
    let fixture = load_fixture().unwrap();
    let mut misses = Vec::new();

    for case in &fixture.smoke {
        let hits = index
            .search(&case.query, &LawFilters::default(), 5)
            .unwrap();
        let rank = rank_of(&hits, &case.identifier, &case.label);
        println!("{:?} rank {:?} :: {}", case.query, rank, show(&hits));
        if rank.is_none() {
            misses.push(case.query.clone());
        }
    }

    println!(
        "smoke: {}/{}",
        fixture.smoke.len() - misses.len(),
        fixture.smoke.len()
    );
    assert!(misses.is_empty(), "missed: {misses:?}");
}

#[test]
#[ignore = "needs data/phlaws.db and data/model"]
fn taglish_meaning_queries_find_a_relevant_article_in_the_top_eight() {
    let index = open_index().unwrap();
    let mut misses = Vec::new();

    for case in &load_fixture().unwrap().taglish {
        let hits = index
            .search(&case.query, &LawFilters::default(), 8)
            .unwrap();
        let rank = case
            .accept
            .iter()
            .filter_map(|a| rank_of(&hits, &a.identifier, &a.label))
            .min();
        println!("{:?} rank {:?} :: {}", case.query, rank, show(&hits));
        if rank.is_none() {
            misses.push(case.query.clone());
        }
    }

    assert!(misses.is_empty(), "missed: {misses:?}");
}

#[test]
#[ignore = "needs data/phlaws.db and data/model"]
fn search_p95_is_under_150_ms_over_100_runs() {
    let index = open_index().unwrap();
    let fixture = load_fixture().unwrap();
    let queries: Vec<&str> = fixture
        .smoke
        .iter()
        .map(|s| s.query.as_str())
        .chain(fixture.taglish.iter().map(|t| t.query.as_str()))
        .collect();
    let mut ms = Vec::new();

    for i in 0..100 {
        let t = Instant::now();
        index
            .search(queries[i % queries.len()], &LawFilters::default(), 8)
            .unwrap();
        ms.push(t.elapsed().as_secs_f64() * 1e3);
    }

    ms.sort_by(f64::total_cmp);
    let (p50, p95) = (ms[49], ms[94]);
    println!(
        "search latency over 100 runs: p50 {p50:.1} ms, p95 {p95:.1} ms, max {:.1} ms",
        ms[99]
    );
    assert!(p95 < 150.0, "p95 {p95} ms");
}

#[test]
#[ignore = "needs data/phlaws.db and data/model"]
fn get_article_returns_the_section_with_its_neighbours() {
    let index = open_index().unwrap();

    let view = index
        .get_article(LawRef::RepublicAct(9262), "Sec. 5")
        .unwrap()
        .unwrap();

    assert_eq!(view.hit.label, "SEC. 5");
    assert_eq!(view.previous.unwrap().label, "SEC. 4");
    assert!(view.next.is_some());
}

#[test]
#[ignore = "needs data/phlaws.db and data/model"]
fn year_filter_drops_laws_outside_the_range() {
    let index = open_index().unwrap();
    let filters = LawFilters {
        year_from: Some(2010),
        year_to: Some(2015),
        law_number: None,
    };

    let hits = index
        .search("data privacy personal information", &filters, 8)
        .unwrap();

    assert_ne!(hits.len(), 0);
    assert!(
        hits.iter()
            .all(|h| h.law.year.is_some_and(|y| (2010..=2015).contains(&y)))
    );
}

#[test]
#[ignore = "needs data/phlaws.db and data/model"]
fn get_article_stitches_every_part_of_a_long_section() {
    let index = open_index().unwrap();

    let view = index
        .get_article(LawRef::RepublicAct(10175), "Section 4")
        .unwrap()
        .unwrap();

    assert_eq!(view.hit.part, 0);
    assert!(view.hit.text.starts_with("SEC. 4. Cybercrime Offenses"));
    assert!(
        view.hit.text.len() > 5000,
        "stitched length {}",
        view.hit.text.len()
    );
    // The dataset splits SEC. 4 at its in-text "Article 355" citation, so the next provision
    // row is that fragment, not SEC. 5; assert only that a different neighbour exists.
    let next = view.next.unwrap();
    assert_ne!(next.text, view.hit.text);
    assert_eq!(next.part, 0);
}
