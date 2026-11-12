//! The statute tool loop in the ask use case (phase-4 §2): tools only when the chat has statutes
//! on and the index is installed, the search event, verbatim assistant content, the 3-call cap
//! with `tool_choice: none`, refused input, law citations, summed usage and one stored answer.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod ask_support;

use ask_support::*;
use kristoff_core::domain::model_request::{
    AnswerEnd, CitationSource, ModelEvent, RawSearchInput, ToolAnswerContent, ToolCall, ToolInput,
};
use kristoff_core::model::{CitationKind, Usage};
use kristoff_core::ports::{ChatStore, UsageMeter};
use kristoff_core::util::time::utc_day;
use kristoff_testkit::builders::chat;
use kristoff_testkit::fakes::{MemoryLawSearch, Script, ScriptedModel};
use kristoff_testkit::world::{START_MS, World};

const TURN: Usage = Usage {
    input: 10,
    output: 5,
    cache_read: 1,
    cache_write: 2,
};

fn search_call(id: &str, query: &str) -> ToolCall {
    let raw = RawSearchInput {
        query: Some(query.into()),
        ..RawSearchInput::default()
    };
    ToolCall {
        id: id.into(),
        input: ToolInput::SearchLaws(raw),
    }
}

/// A turn that says `text`, then stops to make `calls`.
fn tool_turn(text: &str, calls: Vec<ToolCall>) -> Script {
    Script {
        events: vec![ModelEvent::Text(text.into())],
        end: Ok(AnswerEnd {
            usage: TURN,
            stop_reason: Some("tool_use".into()),
            tool_calls: calls,
            assistant_content_json: format!(
                r#"[{{"type":"thinking","thinking":"","signature":"S"}},{{"type":"text","text":"{text}"}}]"#
            ),
        }),
    }
}

fn final_turn(events: Vec<ModelEvent>) -> Script {
    Script {
        events,
        end: Ok(AnswerEnd::finished(TURN, "end_turn")),
    }
}

fn law_citation() -> ModelEvent {
    ModelEvent::Citation(CitationSource::SearchResult {
        source: "https://lawphil.net/fake/9262".into(),
        title: "Republic Act No. 9262 \u{b7} SEC. 5".into(),
        cited_text: "SEC. 5. Acts of Violence".into(),
    })
}

fn with_law(scripts: Vec<Script>) -> World {
    let mut w = world_with_chat(&[]);
    w.law = Some(MemoryLawSearch::sample());
    w.model = ScriptedModel::turns(scripts);
    w
}

fn searching_world() -> World {
    with_law(vec![
        tool_turn(
            "Checking. ",
            vec![search_call("t1", "RA 9262 Sec. 5 acts of violence")],
        ),
        final_turn(vec![
            ModelEvent::Text("Sec. 5 lists the acts.".into()),
            law_citation(),
        ]),
    ])
}

#[test]
fn a_search_runs_between_turns_and_the_answer_is_one_message() {
    let w = searching_world();
    let mut sink = Recorder::default();

    let done = run(&w, "What are the acts?", &mut sink).unwrap();

    let at = |p: &dyn Fn(&String) -> bool| sink.events.iter().position(p).unwrap();
    assert!(at(&|e| e.starts_with("search:")) < at(&|e| e == "text:Sec. 5 lists the acts."));
    assert_eq!(done.message.text, "Checking. Sec. 5 lists the acts. [1]");
    let c = &done.message.citations[0];
    assert_eq!(
        (c.kind, c.page, c.doc_id.clone()),
        (CitationKind::Law, None, None)
    );
    assert_eq!(
        c.source_url.as_deref(),
        Some("https://lawphil.net/fake/9262")
    );
    assert_eq!(w.store.recent_messages(C, 10).unwrap().len(), 2);
}

#[test]
fn every_turn_s_usage_is_summed_into_the_answer_and_the_day() {
    let w = searching_world();

    let done = run(&w, "What are the acts?", &mut Recorder::default()).unwrap();

    let summed = Usage {
        input: 20,
        output: 10,
        cache_read: 2,
        cache_write: 4,
    };
    assert_eq!((done.usage, done.message.usage), (summed, Some(summed)));
    assert_eq!(
        w.store.read_usage(&utc_day(START_MS)).unwrap().today.input,
        20
    );
}

#[test]
fn the_tool_round_carries_the_assistant_content_verbatim_and_the_results() {
    let w = with_law(vec![
        tool_turn(
            "Checking.",
            vec![search_call("t1", "RA 9262 Sec. 5 acts of violence")],
        ),
        final_turn(vec![]),
    ]);

    run(&w, "q", &mut Recorder::default()).unwrap();

    let requests = w.model.requests();
    assert!(requests[0].statutes_on && requests[0].tool_rounds.is_empty());
    let round = &requests[1].tool_rounds[0];
    assert!(
        round
            .assistant_content_json
            .contains(r#""type":"thinking""#)
    );
    assert_eq!(round.answers[0].tool_use_id, "t1");
    let ToolAnswerContent::Results(results) = &round.answers[0].content else {
        panic!("results")
    };
    assert_eq!(results[0].title, "Republic Act No. 9262 \u{b7} SEC. 5");
    assert!(results[0].source.starts_with("https://"));
}

#[test]
fn no_tools_are_sent_when_the_chat_has_statutes_off() {
    let w = with_law(vec![final_turn(vec![ModelEvent::Text("Answer.".into())])]);
    let mut off = chat("00000000000000c2", START_MS - 1000, &[]);
    off.statutes = false;
    w.store.create_chat(&off).unwrap();

    let deps = w.ask();
    let q = kristoff_core::usecases::ask::Question {
        chat_id: &off.id,
        text: "q",
        model: None,
    };
    kristoff_core::usecases::ask::ask(
        &deps,
        q,
        &mut Recorder::default(),
        &kristoff_core::ports::NeverCancelled,
    )
    .unwrap();

    assert!(!w.model.requests()[0].statutes_on);
}

#[test]
fn no_tools_are_sent_without_a_law_index() {
    let w = world_with_chat(&[]);

    run(&w, "q", &mut Recorder::default()).unwrap();

    assert!(!w.model.requests()[0].statutes_on);
}

#[test]
fn after_three_calls_the_next_turn_must_answer_and_extra_calls_are_refused() {
    let calls = |n: &str| {
        vec![
            search_call(&format!("{n}a"), "RA 9262 Sec. 5"),
            search_call(&format!("{n}b"), "RA 9262 Sec. 6"),
        ]
    };
    let w = with_law(vec![
        tool_turn("a", calls("1")),
        tool_turn("b", calls("2")),
        final_turn(vec![]),
    ]);

    run(&w, "q", &mut Recorder::default()).unwrap();

    let requests = w.model.requests();
    assert_eq!(requests.len(), 3);
    assert!(!requests[1].tool_choice_none);
    assert!(requests[2].tool_choice_none, "the cap forces an answer");
    let second = &requests[2].tool_rounds[1].answers;
    assert!(matches!(second[0].content, ToolAnswerContent::Results(_)));
    assert!(matches!(&second[1].content, ToolAnswerContent::Error(m) if m.contains("limit")));
}

#[test]
fn a_bad_tool_input_becomes_an_error_result_and_counts_against_the_cap() {
    let bad = ToolCall {
        id: "t1".into(),
        input: ToolInput::SearchLaws(RawSearchInput::default()),
    };
    let w = with_law(vec![tool_turn("a", vec![bad]), final_turn(vec![])]);
    let mut sink = Recorder::default();

    run(&w, "q", &mut sink).unwrap();

    let answer = &w.model.requests()[1].tool_rounds[0].answers[0];
    assert!(matches!(&answer.content, ToolAnswerContent::Error(m) if m.contains("query")));
    assert!(!sink.events.iter().any(|e| e.starts_with("search:")));
}
