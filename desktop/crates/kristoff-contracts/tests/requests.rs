//! Request bounds: ported from the request schemas in `backend/src/contracts/requests.ts`.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_contracts::requests::{
    ChatPatchRequest, ask, create_doc, cursor, doc_ids, id, link_part, otp_code, project_name,
};
use kristoff_core::error::ErrorCode;
use kristoff_core::ports::ProjectChange;
use serde_json::json;

fn invalid<T: std::fmt::Debug>(result: kristoff_core::error::AppResult<T>) -> bool {
    result.is_err_and(|e| e.code == ErrorCode::ValidationFailed)
}

#[test]
fn accepts_only_six_digit_codes_and_hex_ids() {
    assert!(otp_code("123456").is_ok());
    assert!(invalid(otp_code("12345")) && invalid(otp_code("12345a")));
    assert!(id("0123456789abcdef").is_ok());
    assert!(invalid(id("0123456789ABCDEF")) && invalid(id("../x")));
}

#[test]
fn bounds_upload_requests() {
    assert!(create_doc("a.pdf", 209_715_200).is_ok());
    assert!(invalid(create_doc("a.pdf", 209_715_201)));
    assert!(invalid(create_doc("a.pdf", 0)));
    assert!(invalid(create_doc("", 10)));
    assert!(invalid(create_doc(&"x".repeat(256), 10)));
}

#[test]
fn bounds_cursors_names_parts_and_questions() {
    assert!(cursor(None).is_ok() && invalid(cursor(Some(""))));
    assert!(invalid(cursor(Some(&"a".repeat(513)))));
    assert!(project_name(&"x".repeat(80)).is_ok() && invalid(project_name(&"x".repeat(81))));
    assert!(link_part(Some(999)).is_ok() && invalid(link_part(Some(0))));
    assert!(invalid(link_part(Some(1000))));
    assert!(ask("q", Some("claude-sonnet-5-5")).is_ok() && invalid(ask("", None)));
    assert!(invalid(ask(&"x".repeat(8001), None)));
}

#[test]
fn bounds_attachment_ids() {
    let ok = vec![
        "0123456789abcdef".to_owned(),
        "0123456789abcdef:p12".to_owned(),
    ];
    assert!(doc_ids(&ok).is_ok());
    assert!(invalid(doc_ids(&["0123456789abcdef:p1234".to_owned()])));
    assert!(invalid(doc_ids(&["0123456789abcdef:x1".to_owned()])));
    assert!(invalid(doc_ids(&vec!["0123456789abcdef".to_owned(); 21])));
}

#[test]
fn parses_a_chat_patch_telling_absent_from_null_project() {
    let moved = ChatPatchRequest::parse(json!({"projectId": "0123456789abcdef"})).unwrap();
    let ungrouped = ChatPatchRequest::parse(json!({"projectId": null})).unwrap();
    let pinned = ChatPatchRequest::parse(json!({"pinned": true})).unwrap();

    assert_eq!(
        moved.project,
        Some(ProjectChange::MoveTo("0123456789abcdef".to_owned()))
    );
    assert_eq!(ungrouped.project, Some(ProjectChange::Ungroup));
    assert_eq!((pinned.project, pinned.pinned), (None, Some(true)));
}

#[test]
fn refuses_an_empty_patch_unknown_fields_and_bad_values() {
    for bad in [
        json!({}),
        json!({"colour": "red"}),
        json!({"title": ""}),
        json!({"title": "x".repeat(121)}),
        json!({"pinned": "yes"}),
        json!({"projectId": "nope"}),
        json!({"docIds": ["bad"]}),
    ] {
        assert!(invalid(ChatPatchRequest::parse(bad.clone())), "{bad}");
    }
}
