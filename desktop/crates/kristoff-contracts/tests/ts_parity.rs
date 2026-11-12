//! Knowledge parity with the TypeScript sources (core/dry.md: one source per fact): the error
//! codes, the shared limits, the model labels and the prompts must equal the backend's, read from
//! its files.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_contracts::config::MODEL_LABELS;
use kristoff_contracts::model_config;
use kristoff_core::error::ErrorCode;
use kristoff_core::limits;
use std::path::PathBuf;

fn backend(path: &str) -> String {
    let full = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../backend/src")
        .join(path);
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{}: {e}", full.display()))
}

/// The quoted strings between `start` and the next `]`.
fn quoted_list(text: &str, start: &str) -> Vec<String> {
    let from = text.find(start).unwrap() + start.len();
    let body = &text[from..from + text[from..].find(']').unwrap()];
    body.split('\'')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// The numeric value of `export const NAME = <number>;`.
fn ts_number(text: &str, name: &str) -> u64 {
    let marker = format!("export const {name} = ");
    let from = text
        .find(&marker)
        .unwrap_or_else(|| panic!("{name} missing"))
        + marker.len();
    let raw = &text[from..from + text[from..].find(';').unwrap()];
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    let factors: Vec<u64> = raw
        .split('*')
        .map(|p| p.trim().replace('_', "").parse().unwrap())
        .collect();
    assert!(!digits.is_empty(), "{name} is not numeric");
    factors.iter().product()
}

#[test]
fn error_codes_equal_the_backend_list_in_order() {
    let ts = quoted_list(
        &backend("contracts/errors.ts"),
        "export const ERROR_CODES = [",
    );
    let rust: Vec<&str> = ErrorCode::ALL.iter().map(|c| c.as_str()).collect();

    assert_eq!(rust, ts);
}

#[test]
fn shared_limits_equal_the_backend_values() {
    let ts = backend("domain/limits.ts");
    let pairs: [(&str, u64); 12] = [
        ("MAX_UPLOAD_BYTES", limits::MAX_UPLOAD_BYTES),
        ("MAX_FILE_NAME", limits::MAX_FILE_NAME as u64),
        ("MAX_DOC_TITLE", limits::MAX_DOC_TITLE as u64),
        ("MAX_CHAT_TITLE", limits::MAX_CHAT_TITLE as u64),
        ("MAX_QUESTION", limits::MAX_QUESTION as u64),
        ("MAX_CURSOR", limits::MAX_CURSOR as u64),
        ("MAX_DOC_IDS_CEILING", limits::MAX_DOC_IDS_CEILING as u64),
        ("DOCS_PAGE_SIZE", limits::DOCS_PAGE_SIZE as u64),
        ("CHATS_PAGE_SIZE", limits::CHATS_PAGE_SIZE as u64),
        ("CHAT_MESSAGES_MAX", limits::CHAT_MESSAGES_MAX as u64),
        ("MAX_PROJECT_NAME", limits::MAX_PROJECT_NAME as u64),
        ("MAX_PROJECTS", limits::MAX_PROJECTS as u64),
    ];
    for (name, rust) in pairs {
        assert_eq!(rust, ts_number(&ts, name), "{name}");
    }
    assert_eq!(
        limits::DOC_LINK_TTL_SECONDS,
        i64::try_from(ts_number(&ts, "DOC_LINK_TTL_SECONDS")).unwrap()
    );
    assert_eq!(limits::MAX_PARTS as u64, ts_number(&ts, "MAX_PARTS"));
    assert!(ts.contains(&format!("NEW_CHAT_TITLE = '{}'", limits::NEW_CHAT_TITLE)));
    assert!(ts.contains(&format!("PDF_MAGIC = '{}'", limits::PDF_MAGIC)));
    assert!(ts.contains(&format!(
        "PDF_CONTENT_TYPE = '{}'",
        limits::PDF_CONTENT_TYPE
    )));
}

#[test]
fn model_labels_equal_the_backend_labels() {
    let ts = backend("contracts/model-config.ts");
    for (id, label) in MODEL_LABELS {
        assert!(
            ts.contains(&format!("'{id}': '{label}'")),
            "{id} -> {label}"
        );
    }
}

/// The text of `export const NAME = `...`;` (a template literal without substitutions).
fn ts_template(text: &str, name: &str) -> String {
    let marker = format!("export const {name} = `");
    let from = text
        .find(&marker)
        .unwrap_or_else(|| panic!("{name} missing"))
        + marker.len();
    text[from..from + text[from..].find('`').unwrap()].to_owned()
}

#[test]
fn prompts_and_follow_up_settings_equal_the_backend_ones() {
    let ts = backend("contracts/model-config.ts");
    assert_eq!(
        model_config::SYSTEM_PROMPT,
        ts_template(&ts, "SYSTEM_PROMPT")
    );
    assert_eq!(
        model_config::FOLLOW_UP_PROMPT,
        ts_template(&ts, "FOLLOW_UP_PROMPT")
    );
    assert_eq!(
        u64::from(model_config::FOLLOW_UP_MAX_TOKENS),
        ts_number(&ts, "FOLLOW_UP_MAX_TOKENS")
    );
    assert_eq!(
        model_config::FOLLOW_UP_TIMEOUT_MS,
        ts_number(&ts, "FOLLOW_UP_TIMEOUT_MS")
    );
}

#[test]
fn ask_and_split_limits_equal_the_backend_values() {
    let ts = backend("domain/limits.ts");
    let ask_limits = [
        ("PART_MAX_PAGES", u64::from(limits::PART_MAX_PAGES)),
        ("PART_MAX_BYTES", limits::PART_MAX_BYTES),
        ("MAX_MESSAGE_TEXT", limits::MAX_MESSAGE_TEXT as u64),
        ("MAX_CITATIONS", limits::MAX_CITATIONS as u64),
        ("MAX_QUOTE", limits::MAX_QUOTE as u64),
        ("AUTO_TITLE_CHARS", limits::AUTO_TITLE_CHARS as u64),
        ("MAX_FOLLOW_UPS", limits::MAX_FOLLOW_UPS as u64),
        ("MAX_FOLLOW_UP", limits::MAX_FOLLOW_UP as u64),
        (
            "FOLLOW_UP_ANSWER_CHARS",
            limits::FOLLOW_UP_ANSWER_CHARS as u64,
        ),
    ];
    for (name, rust) in ask_limits {
        assert_eq!(rust, ts_number(&ts, name), "{name}");
    }
}
