//! Wire tests: the exact request bodies against the recorded fixtures (contracts/fixtures/).

#![cfg(test)]

mod support;

use kristoff_anthropic::request::{follow_up_body, messages_body};
use kristoff_contracts::model_config::{FOLLOW_UP_PROMPT, FOLLOW_UP_TIMEOUT_MS, SYSTEM_PROMPT};
use kristoff_core::domain::model_request::{
    AttachedDoc, ModelRequest, ModelTurn, SearchResult, ToolAnswer, ToolRound, TurnBlock,
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

    assert_eq!(messages_body(&req, str_of(&expected, "system")), expected);
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
            results: results
                .iter()
                .map(|r| SearchResult {
                    source: str_of(r, "source").into(),
                    title: str_of(r, "title").into(),
                    text: str_of(&r["content"][0], "text").into(),
                })
                .collect(),
        }],
    }];

    assert_eq!(messages_body(&req, str_of(&expected, "system")), expected);
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
            results: vec![],
        }],
    }];

    let body = messages_body(&req, "s");

    assert_eq!(
        body["messages"][2]["content"][0]["content"],
        json!("No matching statutes were found.")
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
