//! Phase 8 key handling against a scripted local server: the key check's exact request, its
//! outcomes, the shared key slot, and a 401 mapping to `KEY_INVALID`.
#![cfg(test)]

mod support;

use kristoff_anthropic::{
    AnthropicAnswerModel, AnthropicKeyVerifier, AnthropicModel, AnthropicRuntime, ApiConfig,
    SharedKey, build_client,
};
use kristoff_core::error::ErrorCode;
use kristoff_core::ports::{AnswerModel, KeyVerifier, NeverCancelled};
use kristoff_testkit::contract::{assert_code, sample_request};
use std::sync::Arc;
use support::{MockServer, Reply, serve};

const KEY: &str = "sk-ant-api03-candidate";

/// A local server (on its own runtime, kept alive) answering `replies` in order.
fn server(replies: Vec<Reply>) -> (MockServer, tokio::runtime::Runtime) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let server = rt.block_on(serve(replies));
    (server, rt)
}

fn verifier(base_url: &str) -> AnthropicKeyVerifier {
    let rt = Arc::new(AnthropicRuntime::new().unwrap());
    AnthropicKeyVerifier::new(rt, build_client().unwrap(), base_url)
}

#[test]
fn checks_a_key_with_one_models_call_carrying_that_key() {
    let (server, _rt) = server(vec![Reply::json(200, r#"{"data":[],"has_more":true}"#)]);

    verifier(&server.base_url).verify(KEY).unwrap();

    let reqs = server.requests.lock().unwrap().clone();
    assert_eq!(reqs.len(), 1);
    assert_eq!(
        (reqs[0].method.as_str(), reqs[0].path.as_str()),
        ("GET", "/v1/models?limit=1")
    );
    assert_eq!(reqs[0].header("x-api-key"), Some(KEY));
    assert_eq!(reqs[0].header("anthropic-version"), Some("2026-10-21"));
}

#[test]
fn reports_key_invalid_when_anthropic_answers_401_or_403() {
    for status in [401, 403] {
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"x"}}"#;
        let (server, _rt) = server(vec![Reply::json(status, body)]);

        assert_code(
            verifier(&server.base_url).verify(KEY),
            ErrorCode::KeyInvalid,
        );
    }
}

#[test]
fn reports_upstream_unavailable_when_anthropic_keeps_failing() {
    let body = r#"{"type":"error","error":{"type":"api_error","message":"x"}}"#;
    let (server, _rt) = server(vec![Reply::json(500, body), Reply::json(500, body)]);

    assert_code(
        verifier(&server.base_url).verify(KEY),
        ErrorCode::UpstreamUnavailable,
    );
}

fn answer_model(base_url: &str, key: &SharedKey) -> AnthropicAnswerModel {
    let config = ApiConfig {
        base_url: base_url.to_owned(),
        api_key: key.clone(),
    };
    let model = AnthropicModel::new(build_client().unwrap(), config);
    AnthropicAnswerModel::new(Arc::new(AnthropicRuntime::new().unwrap()), model)
}

#[test]
fn maps_a_401_answer_to_key_invalid() {
    let body = r#"{"type":"error","error":{"type":"authentication_error","message":"x"}}"#;
    let (server, _rt) = server(vec![Reply::json(401, body)]);
    let model = answer_model(&server.base_url, &SharedKey::from(KEY));

    let result = model.stream(&sample_request(), &NeverCancelled, &mut |_| {});

    assert_code(result, ErrorCode::KeyInvalid);
}

#[test]
fn sends_the_key_the_shared_slot_holds_at_request_time_and_none_when_empty() {
    let body = r#"{"type":"error","error":{"type":"authentication_error","message":"x"}}"#;
    let (server, _rt) = server(vec![Reply::json(401, body), Reply::json(401, body)]);
    let key = SharedKey::new(None);
    let model = answer_model(&server.base_url, &key);

    let _ = model.stream(&sample_request(), &NeverCancelled, &mut |_| {});
    key.replace(Some("sk-ant-api03-later".into()));
    let _ = model.stream(&sample_request(), &NeverCancelled, &mut |_| {});

    let reqs = server.requests.lock().unwrap().clone();
    assert_eq!(reqs[0].header("x-api-key"), None);
    assert_eq!(reqs[1].header("x-api-key"), Some("sk-ant-api03-later"));
    assert!(key.is_set());
}
