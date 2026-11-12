//! Phase 4 commands: `get_config` features, the per-chat `statutes` toggle and the `search`
//! event on the ask channel (docs/contract.md "Chat: statutes and Config: features").

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use kristoff_app::commands::{ask, chats, session};
use kristoff_core::domain::model_request::{
    AnswerEnd, FollowUps, ModelEvent, RawSearchInput, ToolCall, ToolInput,
};
use kristoff_testkit::fakes::{MemoryLawSearch, Script};
use kristoff_testkit::world::ANSWER_USAGE;
use serde_json::{Value, json};
use support::TestApp;

fn wire<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap()
}

#[test]
fn config_offers_statutes_with_the_law_count_only_once_the_index_is_installed() {
    let app = TestApp::new();

    let before = wire(&session::config(&app.core).unwrap());
    app.core.install_law(Box::new(MemoryLawSearch::sample()));
    let after = wire(&session::config(&app.core).unwrap());

    assert_eq!(
        before["features"],
        json!({ "statutes": false, "lawReady": true })
    );
    assert_eq!(
        after["features"],
        json!({ "statutes": true, "lawCount": 2, "lawReady": false })
    );
}

#[test]
fn a_chat_starts_with_statutes_on_and_update_chat_switches_them_off() {
    let app = TestApp::new();
    let created = wire(&chats::create(&app.core, None).unwrap());
    let id = created["chat"]["id"].as_str().unwrap().to_owned();

    let updated = wire(&chats::update(&app.core, &id, json!({ "statutes": false })).unwrap());

    assert_eq!(created["chat"]["statutes"], true);
    assert_eq!(updated["chat"]["statutes"], false);
    assert_eq!(
        wire(&chats::get(&app.core, &id).unwrap())["chat"]["statutes"],
        false
    );
    assert_eq!(
        wire(&chats::list(&app.core, None).unwrap())["items"][0]["statutes"],
        false
    );
    let bad = chats::update(&app.core, &id, json!({ "statutes": "no" })).unwrap_err();
    assert_eq!(bad.code, "VALIDATION_FAILED");
}

fn searching_app() -> TestApp {
    let raw = RawSearchInput {
        query: Some("RA 9262 Sec. 5".into()),
        ..RawSearchInput::default()
    };
    let call = ToolCall {
        id: "t1".into(),
        input: ToolInput::SearchLaws(raw),
    };
    let first = Script {
        events: vec![],
        end: Ok(AnswerEnd {
            usage: ANSWER_USAGE,
            stop_reason: Some("tool_use".into()),
            tool_calls: vec![call],
            assistant_content_json: "[]".into(),
        }),
    };
    let second = Script {
        events: vec![ModelEvent::Text("Sec. 5.".into())],
        end: Ok(AnswerEnd::finished(ANSWER_USAGE, "end_turn")),
    };
    let app = TestApp::answering_turns(vec![first, second], FollowUps::default());
    app.core.install_law(Box::new(MemoryLawSearch::sample()));
    app
}

#[test]
fn a_statute_search_is_announced_on_the_channel_before_the_text_that_uses_it() {
    let app = searching_app();
    let chat = wire(&chats::create(&app.core, None).unwrap())["chat"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut events = Vec::new();

    ask::ask_now(&app.core, &chat, ("What are the acts?", None), &mut |e| {
        events.push(wire(&e));
        true
    })
    .unwrap();

    let names: Vec<&str> = events
        .iter()
        .map(|e| e["event"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["search", "text", "followups", "done"]);
    assert_eq!(events[0]["data"]["query"], "RA 9262 Sec. 5");
    assert!(events[0]["data"]["count"].as_u64().unwrap() >= 1);
}
