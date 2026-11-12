//! The law index wiring: path resolution with env overrides, the missing-data refusal, and the
//! install that makes the index visible only after it warmed.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::ffi::OsString;
use std::path::Path;

use kristoff_app::law::{DB_ENV, LawPaths, MODEL_ENV, open_law_index};
use kristoff_core::domain::law::LawFilters;
use kristoff_core::error::ErrorCode;
use kristoff_testkit::fakes::MemoryLawSearch;
use support::TestApp;

#[test]
fn bundled_paths_are_used_when_no_override_is_set() {
    let paths = LawPaths::resolve(Path::new("/res"), |_| None);

    assert_eq!(paths.db, Path::new("/res/phlaws.db"));
    assert_eq!(paths.model_dir, Path::new("/res/models/e5-small"));
}

#[test]
fn env_overrides_replace_each_bundled_path() {
    let env = |k: &str| match k {
        DB_ENV => Some(OsString::from("/dev/data/phlaws.db")),
        MODEL_ENV => Some(OsString::from("/dev/data/model")),
        _ => None,
    };

    let paths = LawPaths::resolve(Path::new("/res"), env);

    assert_eq!(paths.db, Path::new("/dev/data/phlaws.db"));
    assert_eq!(paths.model_dir, Path::new("/dev/data/model"));
}

#[test]
fn missing_law_data_is_refused_with_internal() {
    let dir = tempfile::tempdir().unwrap();
    let paths = LawPaths::resolve(dir.path(), |_| None);

    let err = open_law_index(&paths).err().unwrap();

    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.starts_with("LAW_DATA_MISSING"),
        "{}",
        err.message
    );
}

#[test]
fn the_law_index_is_visible_only_after_install() {
    let app = TestApp::new();
    assert!(app.core.law_search().is_none());

    app.core.install_law(Box::new(MemoryLawSearch::sample()));

    let hits = app
        .core
        .law_search()
        .unwrap()
        .search("RA 9262 Sec. 5", &LawFilters::default(), 3)
        .unwrap();
    assert_eq!(hits[0].label, "SEC. 5");
}
