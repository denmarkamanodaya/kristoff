//! The bridge adapters keep core's port contracts (the same suites run on the testkit fakes),
//! against a local server that speaks the recorded wire format.

#![cfg(test)]

mod support;

use kristoff_anthropic::{
    AnthropicAnswerModel, AnthropicFiles, AnthropicFollowUpSuggester, AnthropicFollowUps,
    AnthropicModel, AnthropicRemoteFiles, AnthropicRuntime, ApiConfig, build_client,
};
use kristoff_core::domain::model_request::{
    AnswerEnd, CitationSource, FollowUps, ModelEvent, ModelRequest, ToolCall, ToolInput,
};
use kristoff_core::error::AppResult;
use kristoff_core::ports::{AnswerModel, CancelSignal, FollowUpSuggester, RemoteFiles};
use kristoff_testkit::contract::{
    FollowUpReply, answer_model_contract, assert_code, follow_up_suggester_contract,
    remote_files_contract, sample_request, sample_script,
};
use kristoff_testkit::fakes::{CancelFlag, MemoryLogger, Script};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use support::{Reply, serve};

/// A port adapter plus the runtime its local server runs on (kept alive for the test).
struct Served<A> {
    adapter: A,
    _server: tokio::runtime::Runtime,
}

fn served<A>(
    replies: Vec<Reply>,
    make: impl FnOnce(ApiConfig, Arc<AnthropicRuntime>) -> A,
) -> Served<A> {
    let server_rt = tokio::runtime::Runtime::new().unwrap();
    let server = server_rt.block_on(serve(replies));
    let config = ApiConfig {
        base_url: server.base_url,
        api_key: "test-key".into(),
    };
    Served {
        adapter: make(config, Arc::new(AnthropicRuntime::new().unwrap())),
        _server: server_rt,
    }
}

impl<A: AnswerModel> AnswerModel for Served<A> {
    fn stream(
        &self,
        r: &ModelRequest,
        c: &dyn CancelSignal,
        e: &mut dyn FnMut(ModelEvent),
    ) -> AppResult<AnswerEnd> {
        self.adapter.stream(r, c, e)
    }
}

impl<A: FollowUpSuggester> FollowUpSuggester for Served<A> {
    fn suggest(&self, q: &str, a: &str) -> FollowUps {
        self.adapter.suggest(q, a)
    }
}

impl<A: RemoteFiles> RemoteFiles for Served<A> {
    fn upload(&self, name: &str, pdf: &[u8]) -> AppResult<String> {
        self.adapter.upload(name, pdf)
    }
    fn delete(&self, ids: &[String]) {
        self.adapter.delete(ids);
    }
}

fn frame(data: &serde_json::Value) -> String {
    format!(
        "event: {}\ndata: {data}\n\n",
        data["type"].as_str().unwrap()
    )
}

/// The script as the Messages API would stream it.
/// Each scripted search call as a `tool_use` block whose input streams in two JSON fragments.
fn tool_use_frames(calls: &[ToolCall]) -> String {
    let mut out = String::new();
    for (i, call) in (1..).zip(calls) {
        let ToolInput::SearchLaws(raw) = &call.input else {
            panic!("not scripted: {call:?}")
        };
        let input = json!({ "query": raw.query, "law_number": raw.law_number }).to_string();
        let (a, b) = input.split_at(input.len() / 2);
        out += &frame(
            &json!({ "type": "content_block_start", "index": i, "content_block":
            { "type": "tool_use", "id": call.id, "name": "search_ph_laws", "input": {} } }),
        );
        for part in [a, b] {
            out += &frame(&json!({ "type": "content_block_delta", "index": i,
                "delta": { "type": "input_json_delta", "partial_json": part } }));
        }
        out += &frame(&json!({ "type": "content_block_stop", "index": i }));
    }
    out
}

fn sse_of(script: &Script) -> Vec<u8> {
    let end = script.end.clone().unwrap();
    let u = end.usage;
    let mut out = frame(&json!({ "type": "message_start", "message": { "usage": {
        "input_tokens": u.input, "output_tokens": 1,
        "cache_read_input_tokens": u.cache_read, "cache_creation_input_tokens": u.cache_write } } }));
    out += &frame(
        &json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "text", "text": "" } }),
    );
    for event in &script.events {
        let delta = match event {
            ModelEvent::Text(t) => json!({ "type": "text_delta", "text": t }),
            ModelEvent::Citation(CitationSource::Page {
                document_index,
                document_title,
                start_page,
                end_page_exclusive,
                cited_text,
            }) => {
                json!({ "type": "citations_delta", "citation": { "type": "page_location", "cited_text": cited_text,
                    "document_index": document_index, "document_title": document_title,
                    "start_page_number": start_page, "end_page_number": end_page_exclusive } })
            }
            ModelEvent::Citation(other) => panic!("not scripted: {other:?}"),
        };
        out += &frame(&json!({ "type": "content_block_delta", "index": 0, "delta": delta }));
    }
    out += &tool_use_frames(&end.tool_calls);
    out += &frame(
        &json!({ "type": "message_delta", "delta": { "stop_reason": end.stop_reason }, "usage": { "output_tokens": u.output } }),
    );
    out.into_bytes()
}

fn answer_model(replies: Vec<Reply>) -> Served<AnthropicAnswerModel> {
    served(replies, |config, rt| {
        AnthropicAnswerModel::new(rt, AnthropicModel::new(build_client().unwrap(), config))
    })
}

#[test]
fn the_streaming_adapter_keeps_the_answer_model_contract() {
    answer_model_contract(|script| answer_model(vec![Reply::sse(vec![sse_of(script)])]));
}

#[test]
fn a_cancel_mid_stream_stops_the_adapter_within_a_poll() {
    let bytes = sse_of(&sample_script("end_turn"));
    let mut reply = Reply::sse(vec![bytes[..bytes.len() / 2].to_vec()]);
    reply.stall = Some(Duration::from_secs(30));
    let model = answer_model(vec![reply]);
    let cancel = CancelFlag::default();
    let started = Instant::now();

    let end = model.stream(&sample_request(), &cancel, &mut |_| cancel.cancel());

    assert_code(end, kristoff_core::error::ErrorCode::UpstreamUnavailable);
    assert!(started.elapsed() < Duration::from_secs(5));
}

fn follow_up_reply(reply: &FollowUpReply) -> Reply {
    let (stop, text) = match reply {
        FollowUpReply::Valid(q) => ("end_turn", json!({ "followUps": q }).to_string()),
        FollowUpReply::CutOff(q) => ("max_tokens", json!({ "followUps": q }).to_string()),
        FollowUpReply::Garbled => ("end_turn", "not json".to_owned()),
    };
    let body = json!({ "stop_reason": stop, "content": [{ "type": "text", "text": text }],
        "usage": { "input_tokens": 5, "output_tokens": 9 } });
    Reply::json(200, &body.to_string())
}

#[test]
fn the_haiku_adapter_keeps_the_follow_up_suggester_contract_and_logs_failures() {
    let log = Arc::new(MemoryLogger::default());
    let sink = log.clone();
    follow_up_suggester_contract(|reply| {
        let log = sink.clone();
        served(vec![follow_up_reply(&reply)], move |config, rt| {
            AnthropicFollowUpSuggester::new(
                rt,
                AnthropicFollowUps::new(build_client().unwrap(), config),
                log,
            )
        })
    });
    let reasons: Vec<String> = log
        .warnings()
        .into_iter()
        .map(|(_, reason)| reason)
        .collect();
    assert_eq!(reasons, ["invalid_output", "invalid_output"]);
}

fn remote(replies: Vec<Reply>, log: Arc<MemoryLogger>) -> Served<AnthropicRemoteFiles> {
    served(replies, move |config, rt| {
        AnthropicRemoteFiles::new(
            rt,
            AnthropicFiles::new(build_client().unwrap(), config),
            log,
        )
    })
}

#[test]
fn the_files_adapter_keeps_the_remote_files_contract_and_logs_failed_deletes() {
    let log = Arc::new(MemoryLogger::default());
    remote_files_contract(|| {
        let deleted = |id: &str| {
            Reply::json(
                200,
                &json!({ "type": "file_deleted", "id": id }).to_string(),
            )
        };
        remote(
            vec![
                Reply::json(200, r#"{"type":"file","id":"file_011A"}"#),
                Reply::json(200, r#"{"type":"file","id":"file_011B"}"#),
                deleted("file_011A"),
                deleted("file_011B"),
                Reply::json(
                    404,
                    r#"{"type":"error","error":{"type":"not_found_error","message":"x"}}"#,
                ),
            ],
            log.clone(),
        )
    });
    assert_eq!(
        log.warnings(),
        [(
            "files.delete_failed".to_owned(),
            "UPSTREAM_REJECTED".to_owned()
        )]
    );
}
