//! Fixture parity: every desktop shape in `contracts/fixtures/` deserialises into its Rust type and
//! serialises back to the same JSON (key-sorted), and the domain-to-wire conversions produce the
//! fixture bytes. Fixtures of later phases (`anthropic-*`, `e5-*`) are listed and skipped here.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_contracts::config::{DAILY_QUESTION_CAP, MAX_ATTACHED_DOCS, MODEL_IDS, MODEL_LABELS};
use kristoff_contracts::envelopes::{
    ChatResponse, ChatWithMessagesResponse, ChatsPageResponse, ConfigResponse, DeletedResponse,
    DocResponse, DocUploadResponse, DocsPageResponse, FeaturesDto, LinkResponse, ProjectResponse,
    ProjectsResponse, UploadChunkResponse, UsageResponse,
};
use kristoff_contracts::error::ErrorDto;
use kristoff_contracts::shapes::{CitationDto, ModelOptionDto};
use kristoff_core::model::{Citation, CitationKind};
use kristoff_core::usecases::settings::{ConfigSource, get_config};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::path::PathBuf;

/// Fixtures owned by other phases' tests (`law-`: kristoff-law's smoke test, Phase 3).
const FOREIGN_PREFIXES: [&str; 4] = ["anthropic-", "desktop-anthropic-", "e5-", "law-"];

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../contracts/fixtures")
}

fn load(name: &str) -> Value {
    let text = std::fs::read_to_string(dir().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    serde_json::from_str(&text).unwrap()
}

/// Normalised JSON text: `serde_json`'s map is key-sorted, so equal values print equal bytes.
fn canonical(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}

fn round_trip<T: DeserializeOwned + Serialize>(name: &str) {
    let fixture = load(name);
    let parsed: T =
        serde_json::from_value(fixture.clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
    let back = serde_json::to_value(&parsed).unwrap();
    assert_eq!(canonical(&back), canonical(&fixture), "{name}");
}

/// Every desktop fixture and the check that parses it.
/// A fixture file name and the check that parses it.
type Shape = (&'static str, fn(&str));

const SHAPES: &[Shape] = &[
    ("citation-doc.json", round_trip::<CitationDto>),
    ("citation-law.json", round_trip::<CitationDto>),
    ("desktop-config-response.json", round_trip::<ConfigResponse>),
    (
        "desktop-docs-page-response.json",
        round_trip::<DocsPageResponse>,
    ),
    (
        "desktop-doc-upload-response.json",
        round_trip::<DocUploadResponse>,
    ),
    ("desktop-doc-response.json", round_trip::<DocResponse>),
    ("desktop-link-response.json", round_trip::<LinkResponse>),
    (
        "desktop-chats-page-response.json",
        round_trip::<ChatsPageResponse>,
    ),
    ("desktop-chat-response.json", round_trip::<ChatResponse>),
    (
        "desktop-chat-with-messages-response.json",
        round_trip::<ChatWithMessagesResponse>,
    ),
    ("desktop-usage-response.json", round_trip::<UsageResponse>),
    (
        "desktop-deleted-response.json",
        round_trip::<DeletedResponse>,
    ),
    (
        "desktop-projects-response.json",
        round_trip::<ProjectsResponse>,
    ),
    (
        "desktop-project-response.json",
        round_trip::<ProjectResponse>,
    ),
    (
        "desktop-upload-chunk-response.json",
        round_trip::<UploadChunkResponse>,
    ),
    ("desktop-error-otp-locked.json", round_trip::<ErrorDto>),
    ("desktop-error-doc-not-found.json", round_trip::<ErrorDto>),
];

#[test]
fn every_desktop_fixture_round_trips_byte_for_byte() {
    for (name, check) in SHAPES {
        check(name);
    }
}

#[test]
fn every_fixture_file_is_either_checked_here_or_owned_by_another_phase() {
    for entry in std::fs::read_dir(dir()).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        let foreign = FOREIGN_PREFIXES.iter().any(|p| name.starts_with(p));
        let known = SHAPES.iter().any(|(n, _)| *n == name);
        assert!(
            foreign
                || known
                || !std::path::Path::new(&name)
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("json")),
            "unchecked fixture {name}"
        );
    }
}

#[test]
fn a_law_citation_converts_to_the_fixture() {
    let citation = Citation {
        n: 2,
        doc_id: None,
        doc_title: "RA 9262 · Sec. 5".into(),
        page: None,
        end_page: None,
        quote: "The crime of violence against women and their children is committed through any \
                of the following acts."
            .into(),
        kind: CitationKind::Law,
        source_url: Some("https://lawphil.net/statutes/repacts/ra2004/ra_9262_2004.html".into()),
    };

    let wire = serde_json::to_value(CitationDto::from(&citation)).unwrap();

    assert_eq!(canonical(&wire), canonical(&load("citation-law.json")));
}

#[test]
fn the_app_config_converts_to_the_config_fixture() {
    let source = ConfigSource {
        model_ids: &MODEL_IDS,
        labels: &MODEL_LABELS,
        max_attached_docs: MAX_ATTACHED_DOCS,
        daily_question_cap: DAILY_QUESTION_CAP,
    };
    let config = get_config(&source);
    let models = config.models.iter().map(|m| ModelOptionDto {
        id: m.id.clone(),
        label: m.label.clone(),
    });
    let wire = ConfigResponse {
        model: config.model.clone(),
        models: models.collect(),
        max_attached_docs: config.max_attached_docs,
        daily_question_cap: config.daily_question_cap,
        features: FeaturesDto {
            statutes: true,
            law_count: Some(10_870),
            law_ready: true,
            cases: true,
            case_count: Some(31_180),
        },
    };

    let json = serde_json::to_value(wire).unwrap();

    assert_eq!(
        canonical(&json),
        canonical(&load("desktop-config-response.json"))
    );
}
