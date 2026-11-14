//! The rename's data-folder carry-over (Phase 8): the Kristoff folder is copied into the `LexPH` one
//! on first launch, never over existing `LexPH` data, and the old folder is left as it was.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_app::data_dir::{OLD_IDENTIFIER, carry_over};
use std::fs;
use std::path::Path;

fn old_data(root: &Path) {
    let old = root.join(OLD_IDENTIFIER);
    fs::create_dir_all(old.join("docs/abc")).unwrap();
    fs::write(old.join("kristoff.db"), b"db").unwrap();
    fs::write(old.join("kristoff.db-wal"), b"wal").unwrap();
    fs::write(old.join("docs/abc/whole.pdf"), b"%PDF").unwrap();
}

#[test]
fn copies_the_threads_database_and_pdfs_into_the_new_folder() {
    let root = tempfile::tempdir().unwrap();
    old_data(root.path());
    let new = root.path().join("ph.lexph.desktop");

    let copied = carry_over(&new).unwrap();

    assert!(copied);
    assert_eq!(fs::read(new.join("kristoff.db")).unwrap(), b"db");
    assert_eq!(fs::read(new.join("kristoff.db-wal")).unwrap(), b"wal");
    assert_eq!(fs::read(new.join("docs/abc/whole.pdf")).unwrap(), b"%PDF");
    assert!(
        root.path()
            .join(OLD_IDENTIFIER)
            .join("kristoff.db")
            .exists()
    );
}

#[test]
fn never_copies_over_a_folder_that_already_has_lexph_data() {
    let root = tempfile::tempdir().unwrap();
    old_data(root.path());
    let new = root.path().join("ph.lexph.desktop");
    fs::create_dir_all(&new).unwrap();
    fs::write(new.join("kristoff.db"), b"newer").unwrap();

    let copied = carry_over(&new).unwrap();

    assert!(!copied);
    assert_eq!(fs::read(new.join("kristoff.db")).unwrap(), b"newer");
    assert!(!new.join("docs").exists());
}

#[test]
fn does_nothing_on_a_mac_that_never_ran_kristoff() {
    let root = tempfile::tempdir().unwrap();
    let new = root.path().join("ph.lexph.desktop");

    assert!(!carry_over(&new).unwrap());
    assert!(!new.exists());
}
