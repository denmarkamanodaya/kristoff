//! Builds the exact Messages API request bodies from core's `ModelRequest`. Pure.
//!
//! Ported from `backend/src/adapters/anthropic-answer-model/v1.ts` with file sources (S4), the
//! statute tool and its `search_result` answers (S5, Phase 4 seam), and the D-11 rules: no
//! sampling parameters, no `text` block with citations in a user turn, assistant content sent back
//! unchanged.

use kristoff_contracts::model_config::{
    FOLLOW_UP_MAX_TOKENS, FOLLOW_UP_MODEL, FOLLOW_UP_PROMPT, FOLLOW_UP_SCHEMA, law_tools,
};
use kristoff_core::domain::law::tools::NO_RESULTS_TEXT;
use kristoff_core::domain::model_request::{
    ModelRequest, ModelTurn, SearchResult, ToolAnswer, ToolAnswerContent, ToolRound, TurnBlock,
};
use kristoff_core::model::Role;
use serde_json::{Value, json};

/// The streaming Messages body for one answer call.
#[must_use]
pub fn messages_body(request: &ModelRequest, system: &str) -> Value {
    let mut messages: Vec<Value> = request.turns.iter().map(turn).collect();
    for round in &request.tool_rounds {
        messages.extend(tool_round(round));
    }
    let mut body = json!({
        "model": request.model,
        "max_tokens": request.max_tokens,
        "stream": true,
        "system": system,
        "messages": messages,
    });
    if request.statutes_on {
        body["tools"] = law_tools();
        if request.tool_choice_none {
            body["tool_choice"] = json!({ "type": "none" });
        }
    }
    body
}

/// The non-streaming follow-up body (fixture `anthropic-followups-request.json`).
#[must_use]
pub fn follow_up_body(question: &str, answer_excerpt: &str) -> Value {
    json!({
        "model": FOLLOW_UP_MODEL,
        "max_tokens": FOLLOW_UP_MAX_TOKENS,
        "system": FOLLOW_UP_PROMPT,
        // Haiku thinks by default; with a 400-token cap that can starve the JSON.
        "thinking": { "type": "disabled" },
        "output_config": {
            "effort": "low",
            "format": { "type": "json_schema", "schema": parse_constant(FOLLOW_UP_SCHEMA) }
        },
        "messages": [{
            "role": "user",
            "content": format!(
                "<question>\n{question}\n</question>\n<answer_start>\n{answer_excerpt}\n</answer_start>"
            ),
        }],
    })
}

/// A JSON constant from `kristoff-contracts` (a contracts test proves each one parses).
fn parse_constant(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or(Value::Null)
}

fn role(role: Role) -> &'static str {
    role.as_str()
}

/// A turn of only one text block goes as a plain string, as the TS adapter did.
fn turn(t: &ModelTurn) -> Value {
    if let [TurnBlock::Text(text)] = t.blocks.as_slice() {
        return json!({ "role": role(t.role), "content": text });
    }
    let blocks: Vec<Value> = t.blocks.iter().map(block).collect();
    json!({ "role": role(t.role), "content": blocks })
}

fn block(b: &TurnBlock) -> Value {
    match b {
        TurnBlock::Text(text) => json!({ "type": "text", "text": text }),
        TurnBlock::Doc {
            file_id,
            title,
            is_cache_breakpoint,
        } => {
            let mut doc = json!({
                "type": "document",
                "source": { "type": "file", "file_id": file_id },
                "title": title,
                "citations": { "enabled": true },
            });
            if *is_cache_breakpoint {
                doc["cache_control"] = json!({ "type": "ephemeral" });
            }
            doc
        }
    }
}

fn tool_round(round: &ToolRound) -> [Value; 2] {
    let content = serde_json::from_str::<Value>(&round.assistant_content_json).unwrap_or(json!([]));
    let answers: Vec<Value> = round.answers.iter().map(tool_result).collect();
    [
        json!({ "role": "assistant", "content": content }),
        json!({ "role": "user", "content": answers }),
    ]
}

fn tool_result(answer: &ToolAnswer) -> Value {
    let id = &answer.tool_use_id;
    match &answer.content {
        ToolAnswerContent::Error(message) => {
            json!({ "type": "tool_result", "tool_use_id": id, "content": message, "is_error": true })
        }
        ToolAnswerContent::Results(results) if results.is_empty() => {
            json!({ "type": "tool_result", "tool_use_id": id, "content": NO_RESULTS_TEXT })
        }
        ToolAnswerContent::Results(results) => {
            let content: Vec<Value> = results.iter().map(search_result).collect();
            json!({ "type": "tool_result", "tool_use_id": id, "content": content })
        }
    }
}

fn search_result(result: &SearchResult) -> Value {
    json!({
        "type": "search_result",
        "source": result.source,
        "title": result.title,
        "content": [{ "type": "text", "text": result.text }],
        "citations": { "enabled": true },
    })
}
