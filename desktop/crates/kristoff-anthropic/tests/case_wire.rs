//! Wire tests for the Phase 7 case tools: the tools and prompt sent, the cap, the tool-call
//! mapping and the `search_result_index` read from the stream.

#![cfg(test)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_anthropic::request::messages_body;
use kristoff_anthropic::sse::{SseParser, StreamEvent};
use kristoff_anthropic::tools::tool_call;
use kristoff_contracts::model_config::{answer_system_prompt, case_tools, law_tools};
use kristoff_core::domain::model_request::{
    CitationSource, ModelRequest, ModelTurn, RawCaseSearchInput, RawGetCaseInput, ToolInput,
    TurnBlock,
};
use kristoff_core::model::Role;
use serde_json::{Value, json};

fn request(statutes_on: bool, cases_on: bool) -> ModelRequest {
    ModelRequest {
        model: "claude-sonnet-5-5".into(),
        max_tokens: 64,
        turns: vec![ModelTurn {
            role: Role::User,
            blocks: vec![TurnBlock::Text("q".into())],
        }],
        statutes_on,
        cases_on,
        tool_rounds: vec![],
        tool_choice_none: false,
    }
}

fn body_of(req: &ModelRequest) -> Value {
    messages_body(req, &answer_system_prompt(req.statutes_on, req.cases_on))
}

fn names(body: &Value) -> Vec<String> {
    body["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn statutes_on_sends_all_four_tools_laws_first() {
    let body = body_of(&request(true, true));

    assert_eq!(
        names(&body),
        [
            "search_ph_laws",
            "get_law_article",
            "search_ph_cases",
            "get_case"
        ]
    );
    let mut both = law_tools().as_array().unwrap().clone();
    both.extend(case_tools().as_array().unwrap().iter().cloned());
    assert_eq!(body["tools"], Value::Array(both));
}

#[test]
fn the_system_prompt_carries_the_numbering_and_decision_rules_when_statutes_are_on() {
    let system = body_of(&request(true, true))["system"]
        .as_str()
        .unwrap()
        .to_owned();

    assert!(system.contains("1974 original numbering"), "{system}");
    assert!(system.contains("Art. 266-A after RA 8353"));
    assert!(system.contains("[As enacted]"));
    assert!(system.contains("Cite a decision by its G.R. number"));
    assert!(system.contains("AAA or XXX"));
}

#[test]
fn without_a_case_index_only_the_statute_tools_and_rules_are_sent() {
    let body = body_of(&request(true, false));

    assert_eq!(names(&body), ["search_ph_laws", "get_law_article"]);
    assert!(!body["system"].as_str().unwrap().contains("search_ph_cases"));
}

#[test]
fn statutes_off_sends_no_case_tools_either() {
    let body = body_of(&request(false, false));

    assert!(body.get("tools").is_none(), "{body}");
    assert!(!body["system"].as_str().unwrap().contains("search_ph_cases"));
}

#[test]
fn the_cap_sends_tool_choice_none_with_the_case_tools_still_listed() {
    let mut req = request(false, true);
    req.tool_choice_none = true;

    let body = body_of(&req);

    assert_eq!(body["tool_choice"], json!({ "type": "none" }));
    assert_eq!(names(&body), ["search_ph_cases", "get_case"]);
}

#[test]
fn case_tool_calls_map_to_their_raw_inputs() {
    let search = tool_call(
        "t1",
        "search_ph_cases",
        &json!({"query": "rape", "year_from": 2000, "limit": 3}),
    );
    let get = tool_call(
        "t2",
        "get_case",
        &json!({"case": "G.R. No. 135981", "focus": "self-defense"}),
    );

    assert_eq!(
        search.input,
        ToolInput::SearchCases(RawCaseSearchInput {
            query: Some("rape".into()),
            year_from: Some(2000),
            year_to: None,
            limit: Some(3),
        })
    );
    assert_eq!(
        get.input,
        ToolInput::GetCase(RawGetCaseInput {
            case: Some("G.R. No. 135981".into()),
            focus: Some("self-defense".into()),
        })
    );
}

#[test]
fn a_case_call_with_an_unknown_key_or_no_object_is_malformed() {
    let extra = tool_call("t", "get_case", &json!({"case": "x", "law_number": "RA 1"}));
    let not_object = tool_call("t", "search_ph_cases", &json!("rape"));

    assert_eq!(extra.input, ToolInput::Malformed("get_case".into()));
    assert_eq!(
        not_object.input,
        ToolInput::Malformed("search_ph_cases".into())
    );
}

#[test]
fn a_search_result_citation_keeps_its_search_result_index() {
    let event = json!({"type": "content_block_delta", "index": 0, "delta": {"type": "citations_delta", "citation": {
        "type": "search_result_location", "cited_text": "WHEREFORE", "source": "https://elibrary.judiciary.gov.ph/x",
        "title": "G.R. No. 900001 \u{b7} People v. Testcase (2004)", "search_result_index": 3,
        "start_block_index": 0, "end_block_index": 1 }}});
    let start = json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}});
    let mut parser = SseParser::new();

    let events = parser.feed(format!("data: {start}\n\ndata: {event}\n\n").as_bytes());

    let index = events.iter().find_map(|e| match e {
        StreamEvent::CitationDelta(CitationSource::SearchResult { index, .. }) => Some(*index),
        _ => None,
    });
    assert_eq!(index, Some(Some(3)));
}
