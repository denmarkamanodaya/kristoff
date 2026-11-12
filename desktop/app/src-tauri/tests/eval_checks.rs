//! The eval's pure checks (run in CI): sentence splitting, the uncited statutory claim rule,
//! target scoring, the doc page check, cost and the summary bars. The eval itself is
//! `tests/eval.rs` (ignored).

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod eval_support;

use eval_support::claims::{Cite, names_a_law, sentences, uncited_statutory_sentences};
use eval_support::questions::{Question, Target, load};
use eval_support::recorder::LawCall;
use eval_support::score::{SONNET, cost, doc_page_cited, summary, target_hits};
use kristoff_core::model::Usage;
use serde_json::json;

#[test]
fn abbreviations_do_not_end_a_sentence_but_line_breaks_do() {
    let s = sentences("Under Sec. 26 of R.A. No. 9262, BWS is a defense. It applies.\n## Heading");
    assert_eq!(
        s,
        [
            "Under Sec. 26 of R.A. No. 9262, BWS is a defense.",
            "It applies.",
            "## Heading"
        ]
    );
}

#[test]
fn law_names_are_recognised_in_their_usual_forms() {
    for named in [
        "RA 9262 applies",
        "R.A. No. 10173",
        "Republic Act No. 386",
        "see Sec. 5",
        "Section 12(1)",
        "Art. III",
        "Article 2176 says",
        "under RA9165",
    ] {
        assert!(names_a_law(named), "{named}");
    }
    for plain in [
        "The article says so.",
        "This section explains.",
        "Rules of Court",
    ] {
        assert!(!names_a_law(plain), "{plain}");
    }
}

fn law(n: u32, title: &str) -> Cite {
    Cite::from_json(&json!({ "n": n, "kind": "law", "docTitle": title })).unwrap()
}

fn doc(n: u32) -> Cite {
    Cite::from_json(&json!({ "n": n, "kind": "doc", "docTitle": "People v. X" })).unwrap()
}

#[test]
fn a_sentence_cited_on_it_or_right_after_it_has_a_source() {
    let text = "Section 26 of RA 9262 makes BWS a defense. [1]Victim-survivors incur no liability.\n\n\
                RA 9262 also defines abuse [1].";
    assert_eq!(
        uncited_statutory_sentences(text, &[law(1, "Republic Act No. 9262 \u{b7} SEC. 26")]),
        Vec::<String>::new()
    );
}

#[test]
fn headings_and_label_lines_are_not_claims() {
    let text = "## Related definitions in Sec. 3\n\n**Section 10, RA 7610:**\n\n- **Art. 2176**\n\n\
                ## 1. Unauthorized Disclosure (Sec. 32)\n\nParusa (Sec. 6)\nEmployer liability (Art. 2180)";
    assert_eq!(uncited_statutory_sentences(text, &[]), Vec::<String>::new());
}

#[test]
fn a_short_line_that_states_something_is_still_a_claim() {
    let text = "Sec. 6 punishes it.\n\nRA 9262 penalizes psychological violence with imprisonment and a fine.";
    assert_eq!(uncited_statutory_sentences(text, &[]).len(), 2);
}

#[test]
fn a_doc_citation_counts_as_a_source() {
    let text = "[2]The decision finds him guilty under Section 5(i) of Republic Act No. 9262.";
    assert_eq!(
        uncited_statutory_sentences(text, &[doc(2)]),
        Vec::<String>::new()
    );
}

#[test]
fn a_lead_sentence_is_cited_by_its_list_when_the_list_cites_the_same_law_and_section() {
    let text = "RA 10173 governs data privacy. Sec. 16 lists six rights.\n\n- [1]The right to be informed.\n- [2]The right to object.";
    let cites = [
        law(1, "Republic Act No. 10173 \u{b7} SEC. 16"),
        law(2, "Republic Act No. 10173 \u{b7} SEC. 16"),
    ];
    assert_eq!(
        uncited_statutory_sentences(text, &cites),
        ["RA 10173 governs data privacy."]
    );
}

#[test]
fn a_different_section_or_law_in_the_paragraph_does_not_count() {
    let text = "Sec. 5 lists the acts. [1]The penalty is imprisonment.\n\nUnder RA 7610, Sec. 6 also applies [2].";
    let cites = [
        law(1, "Republic Act No. 9262 \u{b7} SEC. 6"),
        law(2, "Republic Act No. 9262 \u{b7} SEC. 6"),
    ];
    let uncited = uncited_statutory_sentences(text, &cites);
    assert_eq!(
        uncited,
        Vec::<String>::new(),
        "a leading marker right after still cites Sec. 5's sentence"
    );
    let text = "Sec. 5 lists the acts.\n\nUnder RA 7610, Sec. 6 also applies.\n\nSee [1] and [2].";
    assert_eq!(
        uncited_statutory_sentences(text, &cites),
        [
            "Sec. 5 lists the acts.",
            "Under RA 7610, Sec. 6 also applies."
        ]
    );
}

#[test]
fn a_provision_never_cited_anywhere_in_the_answer_still_fails() {
    let text = "Art. 36 of the Family Code covers psychological incapacity.\n\nSec. 16 lists the rights [1].";
    let cites = [law(1, "Republic Act No. 10173 \u{b7} SEC. 16")];
    assert_eq!(
        uncited_statutory_sentences(text, &cites),
        ["Art. 36 of the Family Code covers psychological incapacity."]
    );
}

fn q(kind: &str, targets: &[(&str, &str)]) -> Question {
    Question {
        id: "X".into(),
        kind: kind.into(),
        language: "en".into(),
        text: String::new(),
        targets: targets
            .iter()
            .map(|(i, l)| Target {
                identifier: (*i).into(),
                label: (*l).into(),
            })
            .collect(),
        doc: None,
        doc_page: None,
        must_mention: vec![],
    }
}

/// One search that returned RA 9262 Secs. 1–9 in order.
fn ranked_call() -> LawCall {
    let ranked = (1..=9).map(|n| ("Republic Act No. 9262".to_owned(), format!("SEC. {n}")));
    LawCall {
        tool: "search_ph_laws",
        query: "q".into(),
        provisions: ranked.collect(),
    }
}

fn hits(kind: &str, labels: &[&str]) -> bool {
    let targets: Vec<(&str, &str)> = labels
        .iter()
        .map(|l| ("Republic Act No. 9262", *l))
        .collect();
    target_hits(&q(kind, &targets), &[ranked_call()], &[]).1
}

#[test]
fn a_lookup_needs_every_target_in_a_top_8() {
    assert!(hits("lookup", &["SEC. 8"]));
    assert!(!hits("lookup", &["SEC. 9"]), "rank 9 is outside the top 8");
    assert!(!hits("lookup", &["SEC. 1", "SEC. 9"]));
}

#[test]
fn a_concept_needs_any_target_retrieved_or_cited() {
    let cites = [json!({ "kind": "law", "docTitle": "Republic Act No. 386 \u{b7} ART. 85" })];
    let targets = [
        ("Republic Act No. 9262", "SEC. 9"),
        ("Republic Act No. 386", "ART. 85"),
    ];

    assert!(target_hits(&q("concept", &targets), &[ranked_call()], &cites).1);
    assert!(hits("concept", &["SEC. 9", "SEC. 2"]));
    assert!(!hits("concept", &["SEC. 9"]));
}

#[test]
fn the_doc_page_check_reads_page_ranges() {
    let cites = [
        json!({ "kind": "doc", "page": 1, "endPage": 2 }),
        json!({ "kind": "law", "page": null }),
    ];
    assert!(doc_page_cited(2, &cites));
    assert!(!doc_page_cited(3, &cites));
}

#[test]
fn cost_uses_the_per_million_prices() {
    let usage = Usage {
        input: 1_000_000,
        output: 100_000,
        cache_read: 0,
        cache_write: 400_000,
    };
    assert!((cost(&usage, SONNET) - (2.0 + 1.0 + 1.0)).abs() < 1e-9);
}

#[test]
fn the_summary_applies_the_three_bars() {
    let row = |kind: &str, hit: bool, cost: f64| json!({ "id": "x", "kind": kind, "language": "en", "hit": hit, "uncited": [], "costUsd": cost, "error": null });
    let mut rows: Vec<_> = (0..9).map(|_| row("lookup", true, 0.04)).collect();
    rows.push(row("lookup", false, 0.04));

    let s = summary(&rows);

    assert_eq!(
        s["bars"],
        json!({ "lookupsAtLeast90Percent": true, "zeroUncitedClaims": true, "avgStatuteCostAtMost5Cents": true })
    );
    rows.push(row("concept", true, 0.5));
    assert_eq!(summary(&rows)["bars"]["avgStatuteCostAtMost5Cents"], false);
}

#[test]
fn the_committed_set_has_forty_questions_with_targets() {
    let set = load();
    assert_eq!(set.len(), 40);
    assert!(set.iter().all(|q| !q.targets.is_empty()));
    assert!(
        set.iter()
            .filter(|q| q.kind == "mixed")
            .all(|q| q.doc.is_some() && q.doc_page.is_some())
    );
}
