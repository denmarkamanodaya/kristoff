//! The case tools in the ask loop (phase-7 "Tools"): offered with the Statutes toggle when a case
//! index is installed, one cap across all four tools, the search event's kind, case citations
//! mapped by `search_result_index`, and the "nothing found" text naming the decisions.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod ask_support;

use ask_support::*;
use kristoff_core::domain::model_request::{
    CitationSource, ModelEvent, RawCaseSearchInput, RawGetCaseInput, ToolAnswerContent, ToolCall,
    ToolInput,
};
use kristoff_core::model::CitationKind;
use kristoff_testkit::fakes::{MemoryCaseSearch, MemoryLawSearch, Script, ScriptedModel};
use kristoff_testkit::world::World;

fn case_search(id: &str, query: &str) -> ToolCall {
    let raw = RawCaseSearchInput {
        query: Some(query.into()),
        ..RawCaseSearchInput::default()
    };
    ToolCall {
        id: id.into(),
        input: ToolInput::SearchCases(raw),
    }
}

fn get_case(id: &str, case: &str) -> ToolCall {
    let raw = RawGetCaseInput {
        case: Some(case.into()),
        focus: Some("syndrome".into()),
    };
    ToolCall {
        id: id.into(),
        input: ToolInput::GetCase(raw),
    }
}

fn cited(index: usize) -> ModelEvent {
    ModelEvent::Citation(CitationSource::SearchResult {
        source: "https://example.test/x".into(),
        title: "t".into(),
        cited_text: "quote".into(),
        index: Some(index),
    })
}

fn with_both(scripts: Vec<Script>) -> World {
    let mut w = world_with_chat(&[]);
    w.law = Some(MemoryLawSearch::sample());
    w.cases = Some(MemoryCaseSearch::sample());
    w.model = ScriptedModel::turns(scripts);
    w
}

fn results_of(content: &ToolAnswerContent) -> usize {
    match content {
        ToolAnswerContent::Results(r) => r.len(),
        _ => 0,
    }
}

#[test]
fn case_tools_are_offered_with_statutes_when_a_case_index_exists() {
    let w = with_both(vec![final_turn(vec![])]);

    run(&w, "q", &mut Recorder::default()).unwrap();

    let first = &w.model.requests()[0];
    assert!(first.statutes_on && first.cases_on);
}

#[test]
fn no_case_tools_without_a_case_index() {
    let mut w = world_with_chat(&[]);
    w.law = Some(MemoryLawSearch::sample());

    run(&w, "q", &mut Recorder::default()).unwrap();

    assert!(!w.model.requests()[0].cases_on);
}

#[test]
fn a_case_search_reports_its_kind_and_sends_titled_decisions() {
    let w = with_both(vec![
        tool_turn("a", vec![case_search("t1", "battered woman syndrome")]),
        final_turn(vec![]),
    ]);
    let mut sink = Recorder::default();

    run(&w, "q", &mut sink).unwrap();

    assert!(
        sink.events
            .iter()
            .any(|e| e.starts_with("search:battered woman syndrome:") && e.ends_with(":case"))
    );
    let answer = &w.model.requests()[1].tool_rounds[0].answers[0];
    let ToolAnswerContent::Results(results) = &answer.content else {
        panic!("results")
    };
    assert_eq!(
        results[0].title,
        "G.R. No. 900001 \u{b7} People v. Testcase (2004)"
    );
    assert!(results[0].source.starts_with("https://"));
}

#[test]
fn citations_take_their_kind_from_the_tool_that_sent_the_result() {
    let w = with_both(vec![
        tool_turn(
            "a",
            vec![
                search_call("t1", "RA 9262 Sec. 5"),
                get_case("t2", "G.R. No. 900001"),
            ],
        ),
        final_turn(vec![cited(0), cited(99)]),
    ]);

    let done = run(&w, "q", &mut Recorder::default()).unwrap();

    let round = &w.model.requests()[1].tool_rounds[0];
    let laws = results_of(&round.answers[0].content);
    assert!(laws > 0 && results_of(&round.answers[1].content) == 1);
    let kinds: Vec<_> = done.message.citations.iter().map(|c| c.kind).collect();
    assert_eq!(
        kinds,
        [CitationKind::Law, CitationKind::Law],
        "index 99 is unknown: law"
    );
    let w2 = with_both(vec![
        tool_turn(
            "a",
            vec![
                search_call("t1", "RA 9262 Sec. 5"),
                get_case("t2", "G.R. No. 900001"),
            ],
        ),
        final_turn(vec![cited(laws), cited(laws - 1)]),
    ]);
    let done = run(&w2, "q", &mut Recorder::default()).unwrap();
    let kinds: Vec<_> = done.message.citations.iter().map(|c| c.kind).collect();
    assert_eq!(kinds, [CitationKind::Case, CitationKind::Law]);
}

#[test]
fn the_cap_covers_statute_and_case_calls_together() {
    let w = with_both(vec![
        tool_turn(
            "a",
            vec![search_call("t1", "RA 9262"), case_search("t2", "syndrome")],
        ),
        tool_turn(
            "b",
            vec![
                get_case("t3", "G.R. No. 900002"),
                case_search("t4", "libel"),
            ],
        ),
        final_turn(vec![]),
    ]);

    run(&w, "q", &mut Recorder::default()).unwrap();

    let requests = w.model.requests();
    assert!(!requests[1].tool_choice_none && requests[2].tool_choice_none);
    let second = &requests[2].tool_rounds[1].answers;
    assert!(matches!(second[0].content, ToolAnswerContent::Results(_)));
    assert!(matches!(&second[1].content, ToolAnswerContent::Error(m) if m.contains("limit")));
}

#[test]
fn an_empty_case_search_says_no_decisions_were_found() {
    let w = with_both(vec![
        tool_turn("a", vec![case_search("t1", "zzzz qqqq")]),
        final_turn(vec![]),
    ]);

    run(&w, "q", &mut Recorder::default()).unwrap();

    let answer = &w.model.requests()[1].tool_rounds[0].answers[0];
    assert!(matches!(answer.content, ToolAnswerContent::Nothing(t) if t.contains("decisions")));
}

#[test]
fn statutes_off_offers_no_case_tools_either() {
    let w = with_both(vec![final_turn(vec![])]);
    let mut off = kristoff_testkit::builders::chat(
        "00000000000000c2",
        kristoff_testkit::world::START_MS - 1000,
        &[],
    );
    off.statutes = false;
    kristoff_core::ports::ChatStore::create_chat(&w.store, &off).unwrap();
    let q = kristoff_core::usecases::ask::Question {
        chat_id: &off.id,
        text: "q",
        model: None,
    };

    kristoff_core::usecases::ask::ask(
        &w.ask(),
        q,
        &mut Recorder::default(),
        &kristoff_core::ports::NeverCancelled,
    )
    .unwrap();

    assert!(!w.model.requests()[0].cases_on);
}

#[test]
fn a_case_index_alone_runs_the_loop_and_reaches_the_cap() {
    let mut w = world_with_chat(&[]);
    w.cases = Some(MemoryCaseSearch::sample());
    w.model = ScriptedModel::turns(vec![
        tool_turn(
            "a",
            vec![
                case_search("t1", "a"),
                case_search("t2", "b"),
                case_search("t3", "c"),
            ],
        ),
        final_turn(vec![]),
    ]);

    run(&w, "q", &mut Recorder::default()).unwrap();

    let requests = w.model.requests();
    assert!(!requests[0].statutes_on && requests[0].cases_on);
    assert!(requests[1].tool_choice_none);
}
