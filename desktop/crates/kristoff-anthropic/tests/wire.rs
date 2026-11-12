//! Wire tests: the exact request bodies against the recorded fixtures (contracts/fixtures/).

#![cfg(test)]

mod support;

use kristoff_anthropic::request::{follow_up_body, messages_body};
use kristoff_contracts::model_config::{FOLLOW_UP_PROMPT, FOLLOW_UP_TIMEOUT_MS, SYSTEM_PROMPT};
use kristoff_core::domain::model_request::{
    AttachedDoc, ModelRequest, ModelTurn, SearchResult, ToolAnswer, ToolAnswerContent, ToolRound,
    TurnBlock,
};
use kristoff_core::domain::model_turns::build_model_turns;
use kristoff_core::model::{Message, Role};
use serde_json::{Value, json};
use support::fixture_json;

fn request(model: &str, max_tokens: u32, turns: Vec<ModelTurn>) -> ModelRequest {
    ModelRequest {
        model: model.into(),
        max_tokens,
        turns,
        statutes_on: false,
        tool_rounds: vec![],
        tool_choice_none: false,
    }
}

fn user_text(text: &str) -> ModelTurn {
    ModelTurn {
        role: Role::User,
        blocks: vec![TurnBlock::Text(text.into())],
    }
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap()
}

fn stored(role: &str, text: &str) -> Message {
    Message {
        id: String::new(),
        role: Role::parse(role).unwrap(),
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

#[test]
fn the_desktop_answer_request_matches_its_fixture_from_history_to_wire() {
    let fixture = fixture_json("desktop-anthropic-request.json");
    let r = &fixture["request"];
    let history: Vec<Message> = r["history"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| stored(str_of(m, "role"), str_of(m, "text")))
        .collect();
    let docs: Vec<AttachedDoc> = r["docs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| AttachedDoc {
            doc_id: str_of(d, "docId").into(),
            library_title: str_of(d, "libraryTitle").into(),
            title: str_of(d, "title").into(),
            file_id: str_of(d, "fileId").into(),
            page_offset: u32::try_from(d["pageOffset"].as_u64().unwrap()).unwrap(),
        })
        .collect();
    let max_history = usize::try_from(r["maxHistoryTurns"].as_u64().unwrap()).unwrap();
    let turns = build_model_turns(&history, str_of(r, "question"), &docs, max_history);
    let max_tokens = u32::try_from(r["maxTokens"].as_u64().unwrap()).unwrap();
    let mut expected = fixture["body"].clone();
    expected["system"] = json!(SYSTEM_PROMPT);

    let body = messages_body(
        &request(str_of(r, "model"), max_tokens, turns),
        SYSTEM_PROMPT,
    );

    assert_eq!(body, expected);
}

#[test]
fn a_file_document_request_matches_the_s4_fixture_exactly() {
    let expected = fixture_json("anthropic-file-doc-request.json");
    let doc = &expected["messages"][0]["content"][0];
    let turn = ModelTurn {
        role: Role::User,
        blocks: vec![
            TurnBlock::Doc {
                file_id: str_of(&doc["source"], "file_id").into(),
                title: str_of(doc, "title").into(),
                is_cache_breakpoint: true,
            },
            TurnBlock::Text(str_of(&expected["messages"][0]["content"][1], "text").into()),
        ],
    };

    let body = messages_body(
        &request("claude-sonnet-5-5", 1024, vec![turn]),
        str_of(&expected, "system"),
    );

    assert_eq!(body, expected);
}

#[test]
fn the_first_statute_turn_matches_the_s5_fixture_plus_stream() {
    // Turn 1 of S5 was recorded without streaming; this adapter always streams (lead ruling).
    let mut expected = fixture_json("anthropic-tool-request-turn1.json");
    expected["stream"] = json!(true);
    let mut req = request(
        "claude-sonnet-5-5",
        1024,
        vec![user_text(str_of(&expected["messages"][0], "content"))],
    );
    req.statutes_on = true;

    assert_matches_s5(messages_body(&req, str_of(&expected, "system")), expected);
}

/// Fixture mismatch (reported, fixture unchanged): S5 recorded one tool with `query` and
/// `law_number`; Phase 4 sends two tools and more optional search fields. Everything else must
/// match exactly, and the S5 tool must survive unchanged as a subset of ours.
fn assert_matches_s5(mut body: serde_json::Value, mut expected: serde_json::Value) {
    let ours = body.as_object_mut().unwrap().remove("tools").unwrap();
    let theirs = expected.as_object_mut().unwrap().remove("tools").unwrap();
    assert_eq!(body, expected);
    assert_eq!(ours, kristoff_contracts::model_config::law_tools());
    let (o, t) = (&ours[0], &theirs[0]);
    for key in ["name", "description", "strict"] {
        assert_eq!(o[key], t[key], "{key}");
    }
    for prop in ["query", "law_number"] {
        assert_eq!(
            o["input_schema"]["properties"][prop],
            t["input_schema"]["properties"][prop]
        );
    }
    assert_eq!(o["input_schema"]["required"], t["input_schema"]["required"]);
}

#[test]
fn a_tool_round_sends_assistant_content_verbatim_and_search_results() {
    let mut expected = fixture_json("anthropic-tool-request-turn2.json");
    expected["stream"] = json!(true);
    let results = expected["messages"][2]["content"][0]["content"]
        .as_array()
        .unwrap();
    let mut req = request(
        "claude-sonnet-5-5",
        1024,
        vec![user_text(str_of(&expected["messages"][0], "content"))],
    );
    req.statutes_on = true;
    req.tool_rounds = vec![ToolRound {
        assistant_content_json: expected["messages"][1]["content"].to_string(),
        answers: vec![ToolAnswer {
            tool_use_id: "toolu_01XQjecWVTYPp6oVaCy1QPcS".into(),
            content: ToolAnswerContent::Results(
                results
                    .iter()
                    .map(|r| SearchResult {
                        source: str_of(r, "source").into(),
                        title: str_of(r, "title").into(),
                        text: str_of(&r["content"][0], "text").into(),
                    })
                    .collect(),
            ),
        }],
    }];

    assert_matches_s5(messages_body(&req, str_of(&expected, "system")), expected);
}

#[test]
fn the_follow_up_request_matches_its_fixture_with_the_prompt_inserted() {
    let fixture = fixture_json("anthropic-followups-request.json");
    let mut expected = fixture["body"].clone();
    expected["system"] = json!(FOLLOW_UP_PROMPT);

    let body = follow_up_body(
        str_of(&fixture["input"], "question"),
        str_of(&fixture["input"], "answerExcerpt"),
    );

    assert_eq!(body, expected);
    assert_eq!(fixture["timeoutMs"], json!(FOLLOW_UP_TIMEOUT_MS));
}

#[test]
fn an_empty_search_answers_with_a_plain_text_tool_result() {
    let mut req = request("m", 64, vec![user_text("q")]);
    req.tool_rounds = vec![ToolRound {
        assistant_content_json:
            r#"[{"type":"tool_use","id":"t1","name":"search_ph_laws","input":{}}]"#.into(),
        answers: vec![ToolAnswer {
            tool_use_id: "t1".into(),
            content: ToolAnswerContent::Results(vec![]),
        }],
    }];

    let body = messages_body(&req, "s");

    assert_eq!(
        body["messages"][2]["content"][0]["content"],
        json!(kristoff_core::domain::law::tools::NO_RESULTS_TEXT)
    );
}

#[test]
fn no_sampling_parameters_or_beta_fields_are_ever_sent() {
    let doc = TurnBlock::Doc {
        file_id: "f".into(),
        title: "D".into(),
        is_cache_breakpoint: true,
    };
    let mut req = request(
        "m",
        64,
        vec![ModelTurn {
            role: Role::User,
            blocks: vec![doc, TurnBlock::Text("q".into())],
        }],
    );
    req.statutes_on = true;
    for body in [messages_body(&req, "s"), follow_up_body("q", "a")] {
        let text = body.to_string();
        for key in ["temperature", "top_p", "top_k", "betas"] {
            assert!(!text.contains(&format!("\"{key}\"")), "{key} in {text}");
        }
        let user_blocks = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m["role"] == "user");
        let cited_text = user_blocks
            .filter_map(|m| m["content"].as_array())
            .flatten()
            .any(|b| b["type"] == "text" && b.get("citations").is_some());
        assert!(!cited_text, "a user text block carries citations");
    }
}

/// Any `text` block carrying `citations`, at any depth of `value`.
fn has_cited_text(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            (map.get("type") == Some(&json!("text")) && map.contains_key("citations"))
                || map.values().any(has_cited_text)
        }
        Value::Array(items) => items.iter().any(has_cited_text),
        _ => false,
    }
}

/// A question asked with statutes on, after one tool round with a result and a refused call.
fn tool_round_request() -> ModelRequest {
    let mut req = request("m", 64, vec![user_text("q")]);
    req.statutes_on = true;
    let result = SearchResult {
        source: "https://lawphil.net/ra9262".into(),
        title: "Republic Act No. 9262 \u{b7} SEC. 26".into(),
        text: "SEC. 26. Battered Woman Syndrome as a Defense.".into(),
    };
    req.tool_rounds = vec![ToolRound {
        assistant_content_json: r#"[{"type":"tool_use","id":"t1","name":"search_ph_laws","input":{}},{"type":"tool_use","id":"t2","name":"search_ph_laws","input":{}}]"#.into(),
        answers: vec![
            ToolAnswer { tool_use_id: "t1".into(), content: ToolAnswerContent::Results(vec![result]) },
            ToolAnswer { tool_use_id: "t2".into(), content: ToolAnswerContent::Error("query is required".into()) },
        ],
    }];
    req
}

#[test]
fn no_user_turn_ever_carries_a_cited_text_block_even_inside_tool_results() {
    let body = messages_body(&tool_round_request(), "s");

    let user_turns: Vec<&Value> = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["role"] == "user")
        .collect();

    assert!(!user_turns.iter().any(|m| has_cited_text(m)), "{body}");
    assert_eq!(
        body["messages"][2]["content"][0]["content"][0]["citations"],
        json!({ "enabled": true })
    );
}

#[test]
fn a_refused_call_goes_back_as_an_error_tool_result() {
    let body = messages_body(&tool_round_request(), "s");

    let refused = &body["messages"][2]["content"][1];

    assert_eq!(
        refused,
        &json!({ "type": "tool_result", "tool_use_id": "t2", "content": "query is required", "is_error": true })
    );
    assert!(body["messages"][2]["content"][0].get("is_error").is_none());
}

#[test]
fn statutes_off_sends_no_tools_and_no_tool_choice() {
    let mut req = request("m", 64, vec![user_text("q")]);
    req.tool_choice_none = true;

    let body = messages_body(&req, "s");

    assert!(
        body.get("tools").is_none() && body.get("tool_choice").is_none(),
        "{body}"
    );
}

#[test]
fn the_cap_sends_tool_choice_none_with_the_tools_still_listed() {
    let mut req = tool_round_request();
    req.tool_choice_none = true;

    let body = messages_body(&req, "s");

    assert_eq!(body["tool_choice"], json!({ "type": "none" }));
    assert_eq!(body["tools"], kristoff_contracts::model_config::law_tools());
    assert!(
        messages_body(&tool_round_request(), "s")
            .get("tool_choice")
            .is_none()
    );
}
