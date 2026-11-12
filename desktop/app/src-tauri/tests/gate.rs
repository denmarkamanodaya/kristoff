//! The gate: every command except `verify_otp` fails closed with `UNAUTHENTICATED` while the app
//! is locked, and the unlock ends after 12 h.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use kristoff_app::commands::upload::RawChunk;
use kristoff_app::commands::{CommandResult, ask, chats, docs, projects, session, upload, viewer};
use kristoff_contracts::error::ErrorDto;
use serde_json::json;
use support::TestApp;

const ID: &str = "0123456789abcdef";

fn code_of<T>(result: CommandResult<T>) -> String {
    result
        .err()
        .map_or_else(|| "OK".into(), |e: ErrorDto| e.code)
}

/// Every gated command, called with valid-looking arguments.
fn every_gated_command(app: &TestApp) -> Vec<(&'static str, String)> {
    let a = &app.core;
    vec![
        ("get_config", code_of(session::config(a))),
        ("get_usage", code_of(session::usage(a))),
        ("list_docs", code_of(docs::list(a, None))),
        ("create_doc", code_of(docs::create(a, "a.pdf", 10))),
        (
            "upload_chunk",
            code_of(upload::chunk(a, &RawChunk::default())),
        ),
        ("confirm_doc", code_of(docs::confirm(a, ID))),
        ("rename_doc", code_of(docs::rename(a, ID, "t"))),
        ("get_doc_link", code_of(docs::link(a, ID, None))),
        ("delete_doc", code_of(docs::delete(a, ID))),
        ("list_projects", code_of(projects::list(a))),
        ("create_project", code_of(projects::create(a, "p"))),
        ("rename_project", code_of(projects::rename(a, ID, "p"))),
        ("delete_project", code_of(projects::delete(a, ID))),
        ("list_chats", code_of(chats::list(a, None))),
        ("create_chat", code_of(chats::create(a, None))),
        ("get_chat", code_of(chats::get(a, ID))),
        (
            "update_chat",
            code_of(chats::update(a, ID, json!({"pinned": true}))),
        ),
        ("delete_chat", code_of(chats::delete(a, ID))),
        (
            "ask",
            code_of(ask::ask_now(a, ID, ("q", None), &mut |_| true)),
        ),
        ("cancel_ask", code_of(ask::cancel(a, ID))),
        (
            "open_doc",
            code_of(viewer::doc_window(a, ID, None, Some(1))),
        ),
        (
            "open_external",
            code_of(viewer::external_url(a, "https://lawphil.net/")),
        ),
    ]
}

#[test]
fn every_command_but_verify_otp_is_unauthenticated_while_locked() {
    let app = TestApp::locked();

    for (name, code) in every_gated_command(&app) {
        assert_eq!(code, "UNAUTHENTICATED", "{name}");
    }
}

#[test]
fn the_gated_list_covers_every_command_except_verify_otp() {
    let app = TestApp::locked();
    let mut names: Vec<_> = every_gated_command(&app)
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    names.push("verify_otp");
    names.sort_unstable();
    let mut all = kristoff_contracts::ipc::COMMANDS.to_vec();
    all.sort_unstable();

    assert_eq!(names, all);
}

#[test]
fn a_live_code_unlocks_with_the_local_token_for_twelve_hours() {
    let app = TestApp::locked();

    let session = session::verify(&app.core, &app.code()).unwrap();

    let wire = serde_json::to_value(&session).unwrap();
    assert_eq!(
        wire,
        json!({"token": "local", "expiresAt": "2026-11-12T13:00:00.000Z"})
    );
    assert!(docs::list(&app.core, None).is_ok());
}

#[test]
fn the_unlock_ends_after_twelve_hours() {
    let app = TestApp::unlocked();

    app.clock.advance(12 * 3600 * 1000);

    assert_eq!(code_of(docs::list(&app.core, None)), "UNAUTHENTICATED");
}

#[test]
fn wrong_codes_do_not_unlock_and_the_fifth_locks_with_retry_after() {
    let app = TestApp::locked();
    for _ in 0..4 {
        assert_eq!(code_of(session::verify(&app.core, "000000")), "OTP_INVALID");
    }

    let fifth = session::verify(&app.core, "000000").unwrap_err();

    let wire = serde_json::to_value(&fifth).unwrap();
    assert_eq!(wire["code"], "OTP_LOCKED");
    assert_eq!(wire["retryAfter"], 900);
    assert_eq!(code_of(docs::list(&app.core, None)), "UNAUTHENTICATED");
    assert_eq!(
        code_of(session::verify(&app.core, &app.code())),
        "OTP_LOCKED"
    );
}

#[test]
fn a_replayed_code_is_refused_and_a_malformed_one_is_a_validation_error() {
    let app = TestApp::locked();
    let code = app.code();
    session::verify(&app.core, &code).unwrap();

    assert_eq!(code_of(session::verify(&app.core, &code)), "OTP_REPLAYED");
    assert_eq!(
        code_of(session::verify(&app.core, "12ab56")),
        "VALIDATION_FAILED"
    );
}
