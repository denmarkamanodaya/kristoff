//! The PDF viewer and outside links: `open_doc` windows, the `kristoff://` scheme (gate, path
//! rules, bytes) and the `open_external` host allowlist. Also the request id on failures.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use kristoff_app::commands::upload::RawChunk;
use kristoff_app::commands::{docs, tag_failure, upload, viewer};
use kristoff_app::scheme::{reply, response, scheme_path};
use support::TestApp;
use tauri::http::{StatusCode, Uri, header};

const PDF: &[u8] = b"%PDF-1.7\n%%EOF\n";

fn ready_doc(app: &TestApp) -> String {
    let id = docs::create(&app.core, "Case.pdf", PDF.len() as u64)
        .unwrap()
        .doc
        .id;
    let raw = RawChunk {
        doc_id: Some(id.clone()),
        offset: Some("0".into()),
        body: Some(PDF.to_vec()),
    };
    upload::chunk(&app.core, &raw).unwrap();
    docs::confirm(&app.core, &id).unwrap();
    id
}

#[test]
fn serves_a_ready_doc_as_a_pdf() {
    let app = TestApp::unlocked();
    let id = ready_doc(&app);
    let uri: Uri = format!("kristoff://doc/{id}").parse().unwrap();

    let answer = response(reply(&app.core, &scheme_path(&uri)));

    assert_eq!(answer.status(), StatusCode::OK);
    assert_eq!(answer.headers()[header::CONTENT_TYPE], "application/pdf");
    assert_eq!(answer.body().as_slice(), PDF);
}

#[test]
fn the_scheme_is_forbidden_while_locked() {
    let app = TestApp::unlocked();
    let id = ready_doc(&app);
    app.clock.advance(12 * 3600 * 1000);

    let answer = reply(&app.core, &format!("doc/{id}"));

    assert_eq!(
        (answer.status, answer.body.len()),
        (StatusCode::FORBIDDEN, 0)
    );
}

#[test]
fn the_scheme_answers_404_for_bad_paths_unknown_docs_and_missing_parts() {
    let app = TestApp::unlocked();
    let id = ready_doc(&app);
    let pending = docs::create(&app.core, "p.pdf", 10).unwrap().doc.id;
    let paths = [
        format!("doc/{id}/part-1"),
        "doc/ffffffffffffffff".to_owned(),
        format!("doc/{pending}"),
        format!("doc/{id}/../x"),
        format!("other/{id}"),
    ];

    for path in paths {
        assert_eq!(
            reply(&app.core, &path).status,
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
}

#[test]
fn open_doc_targets_a_labelled_window_at_the_page() {
    let app = TestApp::unlocked();
    let id = ready_doc(&app);

    let window = viewer::doc_window(&app.core, &id, None, Some(3)).unwrap();

    assert_eq!(window.label, format!("doc-{id}"));
    assert_eq!(window.url, format!("kristoff://doc/{id}#page=3"));
    assert_eq!(window.title, "Case");
    let missing = viewer::doc_window(&app.core, "ffffffffffffffff", None, None).unwrap_err();
    assert_eq!(missing.code, "DOC_NOT_FOUND");
    let no_part = viewer::doc_window(&app.core, &id, Some(2), None).unwrap_err();
    assert_eq!(no_part.code, "DOC_NOT_FOUND");
}

#[test]
fn open_external_allows_only_listed_https_hosts() {
    let app = TestApp::unlocked();
    for ok in [
        "https://lawphil.net/statutes/repacts/ra2004/ra_9262_2004.html",
        "https://www.officialgazette.gov.ph/2004/03/08/republic-act-no-9262/",
        "https://elibrary.judiciary.gov.ph/thebookshelf/showdocs/1/1",
        "https://issuances-library.senate.gov.ph/",
    ] {
        assert_eq!(viewer::external_url(&app.core, ok).unwrap(), ok);
    }
    let long = format!("https://lawphil.net/{}", "a".repeat(2100));
    for bad in [
        "http://lawphil.net/",
        "https://evil.example/",
        "file:///etc/passwd",
        long.as_str(),
    ] {
        assert_eq!(
            viewer::external_url(&app.core, bad).unwrap_err().code,
            "VALIDATION_FAILED"
        );
    }
}

#[test]
fn a_failed_command_carries_a_local_request_id() {
    let app = TestApp::locked();

    let err = tag_failure(&app.core, docs::list(&app.core, None)).unwrap_err();

    let wire = serde_json::to_value(&err).unwrap();
    assert_eq!(wire["code"], "UNAUTHENTICATED");
    assert_eq!(wire["requestId"].as_str().map(str::len), Some(26));
}
