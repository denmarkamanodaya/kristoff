//! End-to-end command flows on a real SQLite file and docs folder: upload in chunks, confirm,
//! attach, link, delete (gone from disk and from the chat), projects and the wire shapes.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use kristoff_app::commands::upload::RawChunk;
use kristoff_app::commands::{chats, docs, projects, session, upload};
use serde_json::{Value, json};
use support::TestApp;

/// A real two-page PDF (the lopdf splitter reads it during processing).
fn pdf() -> Vec<u8> {
    kristoff_testkit::pdf::make_pdf(2, |_| 0)
}

fn raw(doc_id: &str, offset: usize, bytes: &[u8]) -> RawChunk {
    RawChunk {
        doc_id: Some(doc_id.to_owned()),
        offset: Some(offset.to_string()),
        body: Some(bytes.to_vec()),
    }
}

fn wire<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap()
}

/// Uploads a real PDF in two chunks, confirms and processes it; returns the doc id.
fn upload_pdf(app: &TestApp) -> String {
    let pdf = pdf();
    let created = docs::create(&app.core, "People v. Cruz.pdf", pdf.len() as u64).unwrap();
    let id = created.doc.id.clone();
    assert_eq!(created.upload_url, format!("kristoff-upload://{id}"));
    assert_eq!(
        wire(&created.upload_headers),
        json!({"Content-Type": "application/pdf"})
    );
    assert_eq!(
        upload::chunk(&app.core, &raw(&id, 0, &pdf[..10]))
            .unwrap()
            .written,
        10
    );
    assert_eq!(
        upload::chunk(&app.core, &raw(&id, 10, &pdf[10..]))
            .unwrap()
            .written,
        pdf.len() as u64
    );
    let confirmed = docs::confirm(&app.core, &id).unwrap();
    assert_eq!(wire(&confirmed)["doc"]["status"], "processing");
    let ready = docs::process(&app.core, &id).unwrap();
    assert_eq!(wire(&ready)["doc"]["status"], "ready");
    assert_eq!(wire(&ready)["doc"]["pageCount"], 2);
    id
}

#[test]
fn uploads_confirms_lists_and_links_a_pdf() {
    let app = TestApp::unlocked();

    let id = upload_pdf(&app);

    let page = wire(&docs::list(&app.core, None).unwrap());
    assert_eq!(page["items"][0]["id"], id.as_str());
    assert_eq!(page["items"][0]["pageCount"], 2);
    assert_eq!(page["nextCursor"], Value::Null);
    let pdf = std::fs::read(app.dir.path().join("docs").join(format!("{id}.pdf"))).unwrap();
    assert_eq!(pdf, self::pdf());
    let link = wire(&docs::link(&app.core, &id, None).unwrap());
    assert_eq!(link["url"], format!("kristoff://doc/{id}"));
    assert_eq!(link["expiresAt"], "2026-11-12T01:05:00.000Z");
}

#[test]
fn refuses_bad_upload_headers_and_a_json_body() {
    let app = TestApp::unlocked();
    let created = docs::create(&app.core, "a.pdf", 10).unwrap();
    let id = created.doc.id;
    let no_offset = RawChunk {
        offset: None,
        ..raw(&id, 0, b"%PDF-")
    };
    let json_body = RawChunk {
        body: None,
        ..raw(&id, 0, b"%PDF-")
    };
    let bad_id = raw("../../x", 0, b"%PDF-");

    for bad in [no_offset, json_body, bad_id] {
        assert_eq!(
            upload::chunk(&app.core, &bad).unwrap_err().code,
            "VALIDATION_FAILED"
        );
    }
    let past_end = upload::chunk(&app.core, &raw(&id, 5, b"%PDF-")).unwrap_err();
    assert_eq!(past_end.code, "UPLOAD_INVALID");
}

#[test]
fn deleting_a_doc_removes_it_from_disk_and_from_every_chat() {
    let app = TestApp::unlocked();
    let id = upload_pdf(&app);
    let chat = chats::create(&app.core, Some(std::slice::from_ref(&id)))
        .unwrap()
        .chat;
    assert_eq!(chat.summary.doc_ids, std::slice::from_ref(&id));

    let deleted = docs::delete(&app.core, &id).unwrap();

    assert_eq!(wire(&deleted), json!({"deleted": true}));
    let after = wire(&chats::get(&app.core, &chat.summary.id).unwrap());
    assert_eq!(after["chat"]["docIds"], json!([]));
    assert_eq!(after["messages"], json!([]));
    let left: Vec<_> = std::fs::read_dir(app.dir.path().join("docs"))
        .unwrap()
        .collect();
    assert!(left.is_empty(), "{left:?}");
    assert_eq!(
        docs::link(&app.core, &id, None).unwrap_err().code,
        "DOC_NOT_FOUND"
    );
}

#[test]
fn moves_a_chat_into_a_project_pins_it_and_ungroups_it_when_the_project_goes() {
    let app = TestApp::unlocked();
    let project = projects::create(&app.core, " Criminal Law ")
        .unwrap()
        .project;
    let chat = chats::create(&app.core, None).unwrap().chat;

    let moved = chats::update(
        &app.core,
        &chat.summary.id,
        json!({"projectId": project.id, "pinned": true}),
    );
    projects::delete(&app.core, &project.id).unwrap();

    let moved = wire(&moved.unwrap());
    assert_eq!(
        (
            moved["chat"]["pinned"].clone(),
            moved["chat"]["projectId"].clone()
        ),
        (json!(true), json!(project.id))
    );
    assert_eq!(project.name, "Criminal Law");
    let after = wire(&chats::list(&app.core, None).unwrap());
    assert_eq!(after["items"][0]["projectId"], Value::Null);
    assert_eq!(
        wire(&projects::list(&app.core).unwrap()),
        json!({"items": []})
    );
}

#[test]
fn reports_config_and_zero_usage_in_the_wire_shape() {
    let app = TestApp::unlocked();

    let config = wire(&session::config(&app.core).unwrap());
    let usage = wire(&session::usage(&app.core).unwrap());

    assert_eq!(config["model"], "claude-sonnet-5-5");
    assert_eq!(config["maxAttachedDocs"], 5);
    let zero = json!({"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0});
    assert_eq!(usage, json!({"today": zero, "total": zero}));
}
