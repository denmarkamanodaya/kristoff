//! The `ask` and `cancel_ask` commands: the channel events match docs/contract.md "Desktop IPC"
//! (`{event, data}`; `done` or `error` last), pre-stream failures reject the command, and a cancel
//! stops the events and stores the partial answer with `stopReason: "error"`.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use kristoff_app::commands::upload::RawChunk;
use kristoff_app::commands::{ask, chats, docs, upload};
use kristoff_core::domain::model_request::{AnswerEnd, CitationSource, FollowUps, ModelEvent};
use kristoff_core::error::{AppError, ErrorCode};
use kristoff_testkit::fakes::Script;
use kristoff_testkit::world::ANSWER_USAGE;
use serde_json::{Value, json};
use support::TestApp;

/// Uploads, confirms and processes a 3-page PDF; returns its id.
fn ready_doc(app: &TestApp) -> String {
    let pdf = kristoff_testkit::pdf::make_pdf(3, |_| 0);
    let id = docs::create(&app.core, "People v. Cruz.pdf", pdf.len() as u64)
        .unwrap()
        .doc
        .id;
    let raw = RawChunk {
        doc_id: Some(id.clone()),
        offset: Some("0".into()),
        body: Some(pdf),
    };
    upload::chunk(&app.core, &raw).unwrap();
    docs::confirm(&app.core, &id).unwrap();
    docs::process(&app.core, &id).unwrap();
    id
}

fn new_chat(app: &TestApp, doc_ids: &[String]) -> String {
    chats::create(&app.core, Some(doc_ids))
        .unwrap()
        .chat
        .summary
        .id
}

/// Runs `ask` and returns the events as wire JSON, plus the command's result.
fn ask_events(app: &TestApp, chat_id: &str, text: &str) -> (Vec<Value>, Result<(), Value>) {
    let mut events = Vec::new();
    let result = ask::ask_now(&app.core, chat_id, (text, None), &mut |e| {
        events.push(serde_json::to_value(e).unwrap());
        true
    });
    (events, result.map_err(|e| serde_json::to_value(e).unwrap()))
}

fn answering(events: Vec<ModelEvent>, stop: &str, follow_ups: Vec<String>) -> TestApp {
    let script = Script {
        events,
        end: Ok(AnswerEnd {
            usage: ANSWER_USAGE,
            stop_reason: Some(stop.into()),
            ..AnswerEnd::default()
        }),
    };
    TestApp::answering(
        script,
        FollowUps {
            questions: follow_ups,
            usage: None,
        },
    )
}

/// An app with one ready doc whose model answers "Held [1]" citing page 2 and suggests "Why?";
/// returns the app, the doc id and the events of one ask.
fn cited_answer() -> (TestApp, String, Vec<Value>) {
    let citation = ModelEvent::Citation(CitationSource::Page {
        document_index: 0,
        document_title: None,
        start_page: 2,
        end_page_exclusive: 3,
        cited_text: "held".into(),
    });
    let events = vec![ModelEvent::Text("Held".into()), citation];
    let app = answering(events, "end_turn", vec!["Why?".into()]);
    let doc = ready_doc(&app);
    let chat = new_chat(&app, std::slice::from_ref(&doc));
    let (events, result) = ask_events(&app, &chat, "What was held?");
    assert_eq!(result, Ok(()));
    (app, doc, events)
}

#[test]
fn streams_text_citation_followups_then_done_last() {
    let (app, doc, events) = cited_answer();

    let names: Vec<&str> = events
        .iter()
        .map(|e| e["event"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["text", "citation", "followups", "done"]);
    assert_eq!(
        events[0],
        json!({ "event": "text", "data": { "text": "Held" } })
    );
    let citation = &events[1]["data"];
    assert_eq!(citation["docId"], doc.as_str());
    assert_eq!(
        (citation["page"].clone(), citation["kind"].clone()),
        (json!(2), json!("doc"))
    );
    let follow_ups = json!({ "event": "followups", "data": { "followUps": ["Why?"] } });
    assert_eq!(events[2], follow_ups);
    assert_eq!(
        app.remote.stored().len(),
        1,
        "the doc was uploaded to the Files API once"
    );
}

#[test]
fn done_carries_the_stored_message_and_the_usage() {
    let (_app, _doc, events) = cited_answer();

    let done = &events[3]["data"];
    assert_eq!(done["message"]["text"], "Held [1]");
    assert_eq!(done["message"]["stopReason"], "end_turn");
    let usage = json!({ "input": 100, "output": 20, "cacheRead": 5, "cacheWrite": 7 });
    assert_eq!(done["usage"], usage);
}

#[test]
fn a_pre_stream_failure_rejects_the_command_with_no_events() {
    let app = TestApp::new();

    let (events, result) = ask_events(&app, "0123456789abcdef", "q");

    assert_eq!(events, Vec::<Value>::new());
    assert_eq!(result.unwrap_err()["code"], "CHAT_NOT_FOUND");
}

#[test]
fn a_failure_after_the_stream_opened_is_an_error_event_last() {
    let failure = AppError::new(ErrorCode::UpstreamUnavailable, "overloaded");
    let script = Script {
        events: vec![ModelEvent::Text("Part".into())],
        end: Err(failure),
    };
    let app = TestApp::answering(script, FollowUps::default());
    let chat = new_chat(&app, &[]);

    let (events, result) = ask_events(&app, &chat, "q");

    assert_eq!(result, Ok(()));
    let last = events.last().unwrap();
    assert_eq!(last["event"], "error");
    assert_eq!(last["data"]["error"]["code"], "UPSTREAM_UNAVAILABLE");
    assert_eq!(
        last["data"]["error"]["requestId"].as_str().map(str::len),
        Some(26)
    );
}

#[test]
fn cancel_stops_the_events_and_stores_the_partial_answer() {
    let app = answering(
        vec![
            ModelEvent::Text("Partial".into()),
            ModelEvent::Text(" more".into()),
        ],
        "end_turn",
        vec![],
    );
    let chat = new_chat(&app, &[]);
    let mut events = Vec::new();

    let result = ask::ask_now(&app.core, &chat, ("q", None), &mut |e| {
        events.push(serde_json::to_value(e).unwrap());
        ask::cancel(&app.core, &chat).unwrap();
        true
    });

    assert!(result.is_ok());
    assert_eq!(
        events,
        [json!({ "event": "text", "data": { "text": "Partial" } })]
    );
    let stored = chats::get(&app.core, &chat).unwrap().messages;
    let answer = serde_json::to_value(stored.last().unwrap()).unwrap();
    assert_eq!(answer["stopReason"], "error");
    assert!(
        answer["text"]
            .as_str()
            .unwrap()
            .starts_with("Partial\n\n_(The answer was interrupted.")
    );
}

#[test]
fn a_closed_channel_cancels_the_ask() {
    let app = answering(
        vec![ModelEvent::Text("a".into()), ModelEvent::Text("b".into())],
        "end_turn",
        vec![],
    );
    let chat = new_chat(&app, &[]);
    let mut sent = 0;

    let result = ask::ask_now(&app.core, &chat, ("q", None), &mut |_| {
        sent += 1;
        false
    });

    assert!(result.is_ok());
    assert_eq!(sent, 1);
    let stored = chats::get(&app.core, &chat).unwrap().messages;
    assert_eq!(
        serde_json::to_value(stored.last().unwrap()).unwrap()["stopReason"],
        "error"
    );
}
