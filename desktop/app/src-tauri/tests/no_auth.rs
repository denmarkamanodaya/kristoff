//! Decision D-17: the desktop app has no authentication. A fresh composition answers commands and
//! serves PDFs at once, with no unlock step, and there is no `verify_otp` command.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use kristoff_app::commands::upload::RawChunk;
use kristoff_app::commands::{docs, projects, session, upload};
use kristoff_app::scheme::reply;
use kristoff_contracts::ipc::COMMANDS;
use support::TestApp;
use tauri::http::StatusCode;

#[test]
fn a_fresh_composition_answers_commands_and_serves_pdfs_with_no_unlock() {
    let app = TestApp::new();
    let pdf = kristoff_testkit::pdf::make_pdf_with_text(1, |_| 0, |_| "Held.".into());

    let id = docs::create(&app.core, "Case.pdf", pdf.len() as u64)
        .unwrap()
        .doc
        .id;
    let raw = RawChunk {
        doc_id: Some(id.clone()),
        offset: Some("0".into()),
        body: Some(pdf.clone()),
    };
    upload::chunk(&app.core, &raw).unwrap();
    docs::confirm(&app.core, &id).unwrap();
    docs::process(&app.core, &id).unwrap();

    assert!(session::config(&app.core).is_ok());
    assert!(projects::create(&app.core, "Torts").is_ok());
    assert_eq!(docs::list(&app.core, None).unwrap().items.len(), 1);
    let served = reply(&app.core, &format!("doc/{id}"));
    assert_eq!((served.status, served.body), (StatusCode::OK, pdf));
}

#[test]
fn there_is_no_verify_otp_command() {
    assert!(!COMMANDS.contains(&"verify_otp"));
}
