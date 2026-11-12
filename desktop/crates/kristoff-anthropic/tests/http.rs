//! HTTP adapters against a scripted local server: headers, exact bodies, retry, cancellation.
#![cfg(test)]

mod support;

use kristoff_anthropic::request::messages_body;
use kristoff_anthropic::sse::StreamEvent;
use kristoff_anthropic::{
    AnthropicFiles, AnthropicFollowUps, AnthropicModel, ApiConfig, CancelToken, ErrorCode,
    build_client,
};
use kristoff_contracts::model_config::{FOLLOW_UP_PROMPT, SYSTEM_PROMPT};
use kristoff_core::model::Usage;
use kristoff_testkit::contract::sample_request;
use std::time::{Duration, Instant};
use support::{Reply, chunks_of, fixture_bytes, serve};

fn config(base_url: &str) -> ApiConfig {
    ApiConfig {
        base_url: base_url.to_owned(),
        api_key: "test-key".into(),
    }
}

fn ask_body() -> serde_json::Value {
    messages_body(&sample_request(), SYSTEM_PROMPT)
}

fn model(base_url: &str) -> AnthropicModel {
    AnthropicModel::new(build_client().unwrap(), config(base_url))
}

#[tokio::test]
async fn streams_with_the_right_headers_and_exact_body() {
    let sse = fixture_bytes("anthropic-file-doc-stream.sse");
    let server = serve(vec![Reply::sse(chunks_of(&sse, 97))]).await;
    let body = ask_body();
    let mut text = String::new();

    let out = model(&server.base_url)
        .stream(&body, &CancelToken::new(), &mut |e| {
            if let StreamEvent::TextDelta(t) = e {
                text.push_str(t);
            }
        })
        .await
        .unwrap();

    let reqs = server.requests.lock().unwrap().clone();
    assert_eq!(reqs.len(), 1);
    let r = &reqs[0];
    assert_eq!(
        (r.method.as_str(), r.path.as_str()),
        ("POST", "/v1/messages")
    );
    assert_eq!(r.header("x-api-key"), Some("test-key"));
    assert_eq!(r.header("anthropic-version"), Some("2026-10-21"));
    assert_eq!(r.header("anthropic-beta"), None);
    assert_eq!(r.header("content-type"), Some("application/json"));
    assert_eq!(r.body, serde_json::to_vec(&body).unwrap());
    assert_eq!(out.stop_reason.as_deref(), Some("end_turn"));
    assert_eq!(
        out.usage,
        Usage {
            input: 26,
            output: 160,
            cache_read: 0,
            cache_write: 10214
        }
    );
    assert!(text.starts_with("The Court ruled that"));
}

#[tokio::test]
async fn retries_once_on_529_honouring_retry_after() {
    let sse = fixture_bytes("anthropic-file-doc-stream.sse");
    let overloaded = Reply::json(
        529,
        r#"{"type":"error","error":{"type":"overloaded_error","message":"x"}}"#,
    )
    .with_header("retry-after", "0");
    let server = serve(vec![overloaded, Reply::sse(vec![sse])]).await;

    let out = model(&server.base_url)
        .stream(&ask_body(), &CancelToken::new(), &mut |_| {})
        .await;

    assert!(out.is_ok(), "{out:?}");
    assert_eq!(server.requests.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn retries_only_once() {
    let busy = || {
        Reply::json(
            503,
            r#"{"type":"error","error":{"type":"api_error","message":"x"}}"#,
        )
        .with_header("retry-after", "0")
    };
    let server = serve(vec![busy(), busy(), busy()]).await;

    let err = model(&server.base_url)
        .stream(&ask_body(), &CancelToken::new(), &mut |_| {})
        .await
        .unwrap_err();

    assert_eq!(
        (err.code, err.status, err.error_type.as_deref()),
        (ErrorCode::UpstreamUnavailable, Some(503), Some("api_error"))
    );
    assert_eq!(server.requests.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn a_400_is_rejected_without_a_retry() {
    let bad = Reply::json(
        400,
        r#"{"type":"error","error":{"type":"invalid_request_error","message":"x"}}"#,
    );
    let server = serve(vec![bad, Reply::json(200, "{}")]).await;

    let err = model(&server.base_url)
        .stream(&ask_body(), &CancelToken::new(), &mut |_| {})
        .await
        .unwrap_err();

    assert_eq!(
        (err.code, err.status),
        (ErrorCode::UpstreamRejected, Some(400))
    );
    assert_eq!(err.error_type.as_deref(), Some("invalid_request_error"));
    assert_eq!(server.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_retry_after_longer_than_20_seconds_fails_at_once() {
    let limited = Reply::json(
        429,
        r#"{"type":"error","error":{"type":"rate_limit_error","message":"x"}}"#,
    )
    .with_header("retry-after", "60");
    let server = serve(vec![limited, Reply::json(200, "{}")]).await;

    let err = model(&server.base_url)
        .stream(&ask_body(), &CancelToken::new(), &mut |_| {})
        .await
        .unwrap_err();

    assert_eq!(err.status, Some(429));
    assert_eq!(server.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn cancelling_mid_stream_stops_at_once_after_the_text_so_far() {
    let sse = fixture_bytes("anthropic-file-doc-stream.sse");
    let head = sse[..1200].to_vec();
    let mut reply = Reply::sse(vec![head]);
    reply.stall = Some(Duration::from_secs(30));
    let server = serve(vec![reply]).await;
    let cancel = CancelToken::new();
    let trigger = cancel.clone();
    let mut texts = 0;
    let started = Instant::now();

    let err = model(&server.base_url)
        .stream(&ask_body(), &cancel, &mut |e| {
            if matches!(e, StreamEvent::TextDelta(_)) {
                texts += 1;
                trigger.cancel();
            }
        })
        .await
        .unwrap_err();

    assert_eq!(err.code, ErrorCode::Cancelled);
    assert!(texts >= 1);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn a_stream_cut_before_message_delta_is_an_invalid_response() {
    let sse = fixture_bytes("anthropic-file-doc-stream.sse");
    let server = serve(vec![Reply::sse(vec![sse[..2000].to_vec()])]).await;

    let err = model(&server.base_url)
        .stream(&ask_body(), &CancelToken::new(), &mut |_| {})
        .await
        .unwrap_err();

    assert_eq!(err.code, ErrorCode::UpstreamInvalidResponse);
}

#[tokio::test]
async fn a_stream_that_ends_cleanly_without_message_delta_is_an_invalid_response() {
    let sse = fixture_bytes("anthropic-file-doc-stream.sse");
    let text = String::from_utf8(sse).unwrap();
    let cut = text.find("event: message_delta").unwrap();
    let server = serve(vec![Reply::sse(vec![text.as_bytes()[..cut].to_vec()])]).await;

    let err = model(&server.base_url)
        .stream(&ask_body(), &CancelToken::new(), &mut |_| {})
        .await
        .unwrap_err();

    assert_eq!(
        (err.code, err.error_type.as_deref()),
        (ErrorCode::UpstreamInvalidResponse, Some("stream_cut"))
    );
}

#[tokio::test]
async fn an_error_event_mid_stream_fails_with_its_type() {
    let raw = b"event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n".to_vec();
    let server = serve(vec![Reply::sse(vec![raw])]).await;

    let err = model(&server.base_url)
        .stream(&ask_body(), &CancelToken::new(), &mut |_| {})
        .await
        .unwrap_err();

    assert_eq!(
        (err.code, err.error_type.as_deref()),
        (ErrorCode::UpstreamUnavailable, Some("overloaded_error"))
    );
}

#[tokio::test]
async fn uploads_a_pdf_as_multipart_and_deletes_by_id() {
    let server = serve(vec![
        Reply::json(
            200,
            r#"{"type":"file","id":"file_011XYZ","filename":"case.pdf"}"#,
        ),
        Reply::json(200, r#"{"type":"file_deleted","id":"file_011XYZ"}"#),
    ])
    .await;
    let files = AnthropicFiles::new(build_client().unwrap(), config(&server.base_url));

    let id = files.upload("case.pdf", b"%PDF-1.4 fake").await.unwrap();
    files.delete(&id).await.unwrap();

    let reqs = server.requests.lock().unwrap().clone();
    assert_eq!(id, "file_011XYZ");
    assert_eq!(
        (reqs[0].method.as_str(), reqs[0].path.as_str()),
        ("POST", "/v1/files")
    );
    assert!(
        reqs[0]
            .header("content-type")
            .unwrap()
            .starts_with("multipart/form-data; boundary=")
    );
    let body = String::from_utf8_lossy(&reqs[0].body);
    assert!(body.contains("Content-Disposition: form-data; name=\"file\"; filename=\"case.pdf\""));
    assert!(body.contains("Content-Type: application/pdf"));
    assert!(body.contains("%PDF-1.4 fake"));
    assert_eq!(reqs[0].header("anthropic-beta"), None);
    assert_eq!(
        (reqs[1].method.as_str(), reqs[1].path.as_str()),
        ("DELETE", "/v1/files/file_011XYZ")
    );
}

#[tokio::test]
async fn follow_ups_send_the_fixture_body_and_read_end_turn_json() {
    let reply = r#"{"stop_reason":"end_turn","content":[{"type":"text","text":"{\"followUps\":[\"What is BWS?\"]}"}],"usage":{"input_tokens":5,"output_tokens":9}}"#;
    let server = serve(vec![Reply::json(200, reply)]).await;
    let follow = AnthropicFollowUps::new(build_client().unwrap(), config(&server.base_url));

    let out = follow
        .suggest(
            "What is estoppel?",
            "Estoppel bars a party [1] from denying.",
        )
        .await;

    assert_eq!(out.follow_ups.questions, vec!["What is BWS?".to_owned()]);
    let fixture = support::fixture_json("anthropic-followups-request.json");
    let mut expected = fixture["body"].clone();
    expected["system"] = serde_json::json!(FOLLOW_UP_PROMPT);
    let sent: serde_json::Value =
        serde_json::from_slice(&server.requests.lock().unwrap()[0].body).unwrap();
    assert_eq!(sent, expected);
}

#[tokio::test]
async fn follow_ups_fail_soft_on_a_server_error() {
    let server = serve(vec![Reply::json(400, "{}")]).await;
    let follow = AnthropicFollowUps::new(build_client().unwrap(), config(&server.base_url));

    let out = follow.suggest("q", "a").await;

    assert_eq!(out.follow_ups.questions, [] as [std::string::String; 0]);
    assert_eq!(out.failure, Some("call_failed"));
}
