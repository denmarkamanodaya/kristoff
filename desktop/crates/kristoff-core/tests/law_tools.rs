//! The statute tool rules: input bounds, status lines, https-only sources, result titles.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::domain::law::tools::{
    LawToolRun, article_results, search_result, status_label, validate,
};
use kristoff_core::domain::law::{ArticleView, LawFilters, LawHit, LawRef, LawSummary};
use kristoff_core::domain::model_request::{RawArticleInput, RawSearchInput, ToolInput};

fn search(raw: RawSearchInput) -> Result<LawToolRun, String> {
    validate(&ToolInput::SearchLaws(raw))
}

fn query(q: &str) -> RawSearchInput {
    RawSearchInput {
        query: Some(q.into()),
        ..RawSearchInput::default()
    }
}

fn hit(label: &str, url: &str, status: &str, truncated: bool) -> LawHit {
    LawHit {
        chunk_id: 1,
        law: LawSummary {
            identifier: "Republic Act No. 9262".into(),
            title: "VAWC".into(),
            number: Some(9262),
            year: Some(2004),
        },
        label: label.into(),
        text: "SEC. 26. Battered Woman Syndrome as a Defense.".into(),
        source_url: url.into(),
        status: status.into(),
        source: "ipfs_philippines_laws".into(),
        edition: None,
        truncated,
        part: 0,
        score: 1.0,
    }
}

#[test]
fn a_valid_search_gets_its_filters_and_the_default_limit() {
    let raw = RawSearchInput {
        law_number: Some("RA 9262".into()),
        year_from: Some(2000),
        ..query(" bws ")
    };

    let run = search(raw).unwrap();

    let filters = LawFilters {
        year_from: Some(2000),
        year_to: None,
        law_number: Some(9262),
    };
    assert_eq!(
        run,
        LawToolRun::Search {
            query: "bws".into(),
            filters,
            limit: 8
        }
    );
}

#[test]
fn out_of_bound_search_input_is_refused_with_a_message() {
    let cases = [
        RawSearchInput::default(),
        query("   "),
        query(&"q".repeat(301)),
        RawSearchInput {
            law_number: Some("x".repeat(41)),
            ..query("q")
        },
        RawSearchInput {
            law_number: Some("the penal code".into()),
            ..query("q")
        },
        RawSearchInput {
            year_from: Some(1899),
            ..query("q")
        },
        RawSearchInput {
            year_to: Some(2101),
            ..query("q")
        },
        RawSearchInput {
            limit: Some(0),
            ..query("q")
        },
        RawSearchInput {
            limit: Some(9),
            ..query("q")
        },
    ];
    for raw in cases {
        assert!(search(raw.clone()).is_err(), "{raw:?}");
    }
    assert!(
        search(RawSearchInput {
            limit: Some(8),
            year_to: Some(2100),
            ..query(&"q".repeat(300))
        })
        .is_ok()
    );
}

#[test]
fn an_article_lookup_needs_a_law_and_a_provision() {
    let ok = validate(&ToolInput::GetLawArticle(RawArticleInput {
        law_number: Some("Constitution".into()),
        article: Some("Art. III Sec. 2".into()),
    }));
    let missing = validate(&ToolInput::GetLawArticle(RawArticleInput {
        law_number: Some("RA 9262".into()),
        article: None,
    }));

    assert_eq!(
        ok,
        Ok(LawToolRun::Article {
            law: LawRef::Constitution,
            article: "Art. III Sec. 2".into()
        })
    );
    assert!(missing.is_err());
    assert!(validate(&ToolInput::Malformed("search_ph_laws".into())).is_err());
    assert!(validate(&ToolInput::Unknown("web_search".into())).is_err());
}

#[test]
fn law_status_values_map_to_status_lines() {
    assert_eq!(status_label("current"), None);
    assert_eq!(status_label("Repealed"), Some("Repealed"));
    assert_eq!(
        status_label("partially_repealed"),
        Some("Partially repealed")
    );
    assert_eq!(status_label("amended"), Some("Amended"));
    assert_eq!(
        status_label("declared-unconstitutional"),
        Some("Declared unconstitutional")
    );
    assert_eq!(status_label("something new"), None);
}

#[test]
fn a_result_is_titled_by_law_and_label_and_starts_with_its_status_line() {
    let r = search_result(&hit(
        "SEC. 26",
        "https://lawphil.net/ra9262",
        "repealed",
        false,
    ))
    .unwrap();
    let cut = search_result(&hit(
        "SEC. 26",
        "https://lawphil.net/ra9262",
        "current",
        true,
    ))
    .unwrap();
    let plain = search_result(&hit(
        "SEC. 26",
        "https://lawphil.net/ra9262",
        "current",
        false,
    ))
    .unwrap();

    assert_eq!(r.title, "Republic Act No. 9262 \u{b7} SEC. 26");
    assert_eq!(r.source, "https://lawphil.net/ra9262");
    assert!(r.text.starts_with("[Repealed] SEC. 26."));
    assert!(cut.text.starts_with("[Text may be cut off] "));
    assert!(plain.text.starts_with("SEC. 26."));
}

#[test]
fn an_edition_label_comes_before_the_status_and_cut_off_lines() {
    let mut code = hit("ART. 335", "https://lawphil.net/act3815", "repealed", true);
    code.edition = Some("As enacted".into());

    let r = search_result(&code).unwrap();

    assert!(r.text.starts_with("[As enacted] SEC. 26."), "{}", r.text);
}

#[test]
fn a_passage_without_an_https_source_is_left_out() {
    assert_eq!(
        search_result(&hit("SEC. 1", "http://lawphil.net/x", "current", false)),
        None
    );
    assert_eq!(search_result(&hit("SEC. 1", "", "current", false)), None);
}

#[test]
fn an_article_comes_first_then_its_neighbours() {
    let view = ArticleView {
        hit: hit("SEC. 26", "https://a/26", "current", false),
        previous: Some(hit("SEC. 25", "https://a/25", "current", false)),
        next: Some(hit("SEC. 27", "http://a/27", "current", false)),
    };

    let titles: Vec<String> = article_results(&view)
        .into_iter()
        .map(|r| r.title)
        .collect();

    assert_eq!(
        titles,
        [
            "Republic Act No. 9262 \u{b7} SEC. 26",
            "Republic Act No. 9262 \u{b7} SEC. 25"
        ]
    );
}
