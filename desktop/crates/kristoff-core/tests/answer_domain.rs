//! Model turns and citation mapping (pure domain rules).

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::domain::citation::map_citation;
use kristoff_core::domain::model_request::{AttachedDoc, CitationSource, TurnBlock};
use kristoff_core::domain::model_turns::build_model_turns;
use kristoff_core::model::{CitationKind, Message, Role};

fn msg(role: Role, text: &str) -> Message {
    Message {
        id: String::new(),
        role,
        text: text.into(),
        citations: vec![],
        usage: None,
        model: None,
        stop_reason: None,
        at_ms: 0,
        duration_ms: None,
        follow_ups: vec![],
    }
}

fn attached(n: u32, offset: u32) -> AttachedDoc {
    AttachedDoc {
        doc_id: format!("doc-{n}"),
        library_title: "Genosa".into(),
        title: format!("Genosa part {n}"),
        file_id: format!("file_{n}"),
        page_offset: offset,
    }
}

fn texts(turns: &[kristoff_core::domain::model_request::ModelTurn]) -> Vec<(Role, String)> {
    turns
        .iter()
        .map(|t| match t.blocks.last() {
            Some(TurnBlock::Text(text)) => (t.role, text.clone()),
            _ => panic!("a turn ends with text"),
        })
        .collect()
}

#[test]
fn keeps_the_last_20_from_a_user_turn_merges_roles_and_fills_empty_text() {
    let mut history = vec![msg(Role::Assistant, "dropped")];
    for i in 0..10 {
        history.push(msg(Role::User, &format!("u{i}")));
        history.push(msg(Role::Assistant, if i == 1 { "" } else { "a" }));
    }
    history.push(msg(Role::User, "u9b"));

    let turns = build_model_turns(&history, "Q", &[], 20);

    let t = texts(&turns);
    assert_eq!(t[0], (Role::User, "u1".into()));
    assert_eq!(t[1], (Role::Assistant, "(no answer)".into()));
    assert_eq!(t.last().unwrap(), &(Role::User, "u9b\n\nQ".into()));
    assert!(t.windows(2).all(|w| w[0].0 != w[1].0));
}

#[test]
fn docs_go_only_in_the_first_turn_with_the_cache_breakpoint_on_the_last() {
    let history = vec![msg(Role::User, "x"), msg(Role::Assistant, "y")];

    let turns = build_model_turns(&history, "Q", &[attached(1, 0), attached(2, 100)], 20);

    let first = &turns[0].blocks;
    assert_eq!(first.len(), 3);
    assert!(matches!(
        &first[0],
        TurnBlock::Doc {
            is_cache_breakpoint: false,
            ..
        }
    ));
    assert!(
        matches!(&first[1], TurnBlock::Doc { is_cache_breakpoint: true, file_id, .. } if file_id == "file_2")
    );
    assert!(turns[1..].iter().all(|t| t.blocks.len() == 1));
}

#[test]
fn no_history_window_sends_only_the_question() {
    let turns = build_model_turns(&[msg(Role::User, "x")], "Q", &[], 0);
    assert_eq!(texts(&turns), [(Role::User, "Q".to_owned())]);
}

#[test]
fn a_part_citation_maps_to_original_pages_with_an_inclusive_end() {
    let source = CitationSource::Page {
        document_index: 1,
        document_title: Some("ignored".into()),
        start_page: 3,
        end_page_exclusive: 5,
        cited_text: "a\r\nb\rc".into(),
    };

    let c = map_citation(&source, &[attached(1, 0), attached(2, 100)], 2);

    assert_eq!(
        (c.n, c.doc_id.as_deref(), c.doc_title.as_str()),
        (2, Some("doc-2"), "Genosa")
    );
    assert_eq!(
        (c.page, c.end_page, c.quote.as_str()),
        (Some(103), Some(104), "a\nb\nc")
    );
    assert_eq!((c.kind, c.source_url), (CitationKind::Doc, None));
}

#[test]
fn an_unknown_index_keeps_the_reported_title_and_caps_title_and_quote() {
    let source = CitationSource::Page {
        document_index: 9,
        document_title: Some("\u{f1}".repeat(300)),
        start_page: 1,
        end_page_exclusive: 2,
        cited_text: "q".repeat(5000),
    };

    let c = map_citation(&source, &[], 1);

    assert_eq!(c.doc_id, None);
    assert_eq!((c.doc_title.chars().count(), c.quote.len()), (200, 4000));
}

#[test]
fn a_search_result_citation_is_a_law_with_its_source_link() {
    let source = CitationSource::SearchResult {
        source: "https://lawphil.net/x#sec26".into(),
        title: "RA 9262 \u{b7} Sec. 26".into(),
        cited_text: "SECTION 26.".into(),
    };

    let c = map_citation(&source, &[attached(1, 0)], 1);

    assert_eq!((c.kind, c.doc_id, c.page), (CitationKind::Law, None, None));
    assert_eq!(c.source_url.as_deref(), Some("https://lawphil.net/x#sec26"));
}
