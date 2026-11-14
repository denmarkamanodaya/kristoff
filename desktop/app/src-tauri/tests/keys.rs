//! Phase 8 commands end to end on a real composition (fakes for Claude and the key store): key
//! status, save, remove, the `aiReady` flag, ask without a key, and the daily limit setting.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use kristoff_app::commands::viewer::external_url;
use kristoff_app::commands::{ask, chats, keys, session};
use serde_json::{Value, json};
use support::{TEST_KEY, TestApp};

fn wire<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap()
}

fn ai_ready(app: &TestApp) -> Value {
    wire(&session::config(&app.core).unwrap())["features"]["aiReady"].clone()
}

#[test]
fn reports_the_loaded_key_by_its_last_four_characters_only() {
    let app = TestApp::new();

    let status = wire(&keys::key_status(&app.core).unwrap());

    assert_eq!(status, json!({ "configured": true, "hint": "AB12" }));
    assert_eq!(ai_ready(&app), json!(true));
}

#[test]
fn removing_the_key_turns_ai_off_and_ask_answers_key_missing() {
    let app = TestApp::new();
    let chat = wire(&chats::create(&app.core, None).unwrap())["chat"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let removed = wire(&keys::remove_key(&app.core).unwrap());
    let refused = ask::ask_now(&app.core, &chat, ("q", None), &mut |_| true).unwrap_err();

    assert_eq!(removed, json!({ "configured": false, "hint": null }));
    assert_eq!(ai_ready(&app), json!(false));
    assert_eq!(refused.code, "KEY_MISSING");
}

#[test]
fn saving_a_working_key_turns_ai_back_on() {
    let app = TestApp::new();
    keys::remove_key(&app.core).unwrap();

    let saved = wire(&keys::set_key(&app.core, &format!(" {TEST_KEY} ")).unwrap());

    assert_eq!(saved, json!({ "configured": true, "hint": "AB12" }));
    assert_eq!(ai_ready(&app), json!(true));
}

#[test]
fn a_rejected_or_malformed_key_answers_key_invalid_and_changes_nothing() {
    let app = TestApp::new();

    for bad in ["not-a-key", "sk-ant-api03-revokedZZ99"] {
        let err = keys::set_key(&app.core, bad).unwrap_err();
        assert_eq!(err.code, "KEY_INVALID");
    }

    assert_eq!(
        wire(&keys::key_status(&app.core).unwrap())["hint"],
        json!("AB12")
    );
}

#[test]
fn the_daily_limit_setting_changes_what_config_reports() {
    let app = TestApp::new();

    let before = wire(&keys::settings(&app.core).unwrap());
    let after = wire(&keys::update(&app.core, 7).unwrap());
    let refused = keys::update(&app.core, 0).unwrap_err();

    assert_eq!(before, json!({ "dailyQuestionCap": 100 }));
    assert_eq!(after, json!({ "dailyQuestionCap": 7 }));
    assert_eq!(refused.code, "VALIDATION_FAILED");
    assert_eq!(
        wire(&session::config(&app.core).unwrap())["dailyQuestionCap"],
        json!(7)
    );
}

#[test]
fn the_key_page_and_the_dataset_credit_may_open_in_the_browser() {
    for url in [
        "https://console.anthropic.com/settings/keys",
        "https://huggingface.co/datasets/mongramosjr/philippine-omnicorpus",
    ] {
        assert_eq!(external_url(url).unwrap(), url);
    }
    assert!(external_url("https://evil.example/console.anthropic.com").is_err());
}

#[test]
fn a_key_rejected_mid_ask_reaches_the_web_as_a_key_invalid_error_event() {
    use kristoff_core::domain::model_request::FollowUps;
    use kristoff_core::error::{AppError, ErrorCode};
    use kristoff_testkit::fakes::Script;
    let rejected = Script {
        events: vec![],
        end: Err(AppError::new(
            ErrorCode::KeyInvalid,
            "Anthropic rejected the Claude API key",
        )),
    };
    let app = TestApp::answering(rejected, FollowUps::default());
    let chat = wire(&chats::create(&app.core, None).unwrap())["chat"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut events = Vec::new();

    ask::ask_now(&app.core, &chat, ("q", None), &mut |e| {
        events.push(wire(&e));
        true
    })
    .unwrap();

    let last = events.last().unwrap();
    assert_eq!(last["event"], json!("error"));
    assert_eq!(last["data"]["error"]["code"], json!("KEY_INVALID"));
}
