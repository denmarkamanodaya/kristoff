//! The law index is visible from the first `get_config` after start-up (the gate used to hide
//! the race): setup opens and installs the index synchronously, and only warms it in the
//! background. A search before the warm-up ends waits for it.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use kristoff_app::commands::session;
use kristoff_app::law::{LawPaths, open_and_install, open_law_index};
use kristoff_core::domain::law::LawFilters;
use kristoff_testkit::fakes::MemoryLawSearch;
use serde_json::json;
use std::path::{Path, PathBuf};
use support::TestApp;

/// A tiny but valid `phlaws.db` with `law_count` 3, plus a model folder whose weights are never
/// read (the embedder loads lazily, on the first warm-up or search).
fn tiny_law_data(dir: &Path) -> LawPaths {
    let db = dir.join("phlaws.db");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(kristoff_law::schema::CREATE).unwrap();
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('law_count', '3')",
        [],
    )
    .unwrap();
    drop(conn);
    let model_dir = dir.join("model");
    std::fs::create_dir_all(&model_dir).unwrap();
    std::fs::write(model_dir.join("model.safetensors"), b"not loaded at open").unwrap();
    LawPaths { db, model_dir }
}

fn features(app: &TestApp) -> serde_json::Value {
    serde_json::to_value(session::config(&app.core).unwrap()).unwrap()["features"].clone()
}

#[test]
fn config_right_after_setup_reports_statutes_when_law_data_exists() {
    let app = TestApp::new();
    let data = tempfile::tempdir().unwrap();
    let paths = tiny_law_data(data.path());

    open_and_install(&app.core, &paths).unwrap();

    assert_eq!(
        features(&app),
        json!({"statutes": true, "lawCount": 3, "lawReady": false})
    );
}

#[test]
fn config_reports_no_statutes_when_the_law_data_is_missing() {
    let app = TestApp::new();
    let empty = tempfile::tempdir().unwrap();

    let opened = open_and_install(&app.core, &LawPaths::resolve(empty.path(), |_| None));

    assert!(opened.is_err());
    assert_eq!(features(&app), json!({"statutes": false, "lawReady": true}));
}

#[test]
fn law_ready_flips_once_the_warm_up_has_run() {
    let app = TestApp::new();
    app.core.install_law(Box::new(MemoryLawSearch::sample()));
    assert_eq!(features(&app)["lawReady"], false);

    app.core.warm_law().unwrap();

    assert_eq!(features(&app)["lawReady"], true);
}

#[test]
fn law_ready_is_set_even_when_the_warm_up_fails() {
    let app = TestApp::new();
    let data = tempfile::tempdir().unwrap();
    open_and_install(&app.core, &tiny_law_data(data.path())).unwrap();

    let warmed = app.core.warm_law();

    assert!(warmed.is_err(), "the dummy weights cannot load");
    assert_eq!(features(&app)["lawReady"], true);
}

fn real_data() -> LawPaths {
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    LawPaths {
        db: data.join("phlaws.db"),
        model_dir: data.join("model"),
    }
}

#[test]
#[ignore = "needs desktop/data (phlaws.db + model); run by ci-local when present"]
fn the_real_index_opens_fast_and_a_search_before_warm_up_waits_for_it() {
    let started = std::time::Instant::now();
    let index = open_law_index(&real_data()).unwrap();
    let open_ms = started.elapsed().as_millis();

    let hits = index
        .search("RA 9262 Sec. 5", &LawFilters::default(), 3)
        .unwrap();

    eprintln!(
        "law index open: {open_ms} ms; first search (cold): {} ms",
        started.elapsed().as_millis()
    );
    assert!(open_ms < 200, "open took {open_ms} ms");
    assert_ne!(hits.len(), 0);
}
