//! `ProjectStore` contract.

use super::assert_code;
use crate::builders::project;
use kristoff_core::error::ErrorCode;
use kristoff_core::ports::{ProjectRename, ProjectStore};

const A: &str = "0000000000000001";
const B: &str = "0000000000000002";
const C: &str = "0000000000000003";

/// Runs every `ProjectStore` promise against fresh stores from `make`.
pub fn project_store_contract<S: ProjectStore>(make: impl Fn() -> S) {
    creates_reads_and_lists(&make());
    refuses_a_project_past_the_cap(&make());
    renames_at_the_read_version_only(&make());
    deletes_and_then_reports_missing(&make());
}

fn creates_reads_and_lists(store: &impl ProjectStore) {
    store.create_project(&project(A, "Torts"), 50).unwrap();
    store.create_project(&project(B, "Crim"), 50).unwrap();

    let mut names: Vec<_> = store
        .list_projects()
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    names.sort();

    assert_eq!(store.get_project(A).unwrap().value, project(A, "Torts"));
    assert_eq!(names, ["Crim", "Torts"]);
    assert_code(
        store.create_project(&project(A, "Again"), 50),
        ErrorCode::Internal,
    );
    assert_code(store.get_project(C), ErrorCode::ProjectNotFound);
}

fn refuses_a_project_past_the_cap(store: &impl ProjectStore) {
    store.create_project(&project(A, "One"), 2).unwrap();
    store.create_project(&project(B, "Two"), 2).unwrap();

    assert_code(
        store.create_project(&project(C, "Three"), 2),
        ErrorCode::ProjectLimitReached,
    );
    assert_eq!(store.list_projects().unwrap().len(), 2);
}

fn renames_at_the_read_version_only(store: &impl ProjectStore) {
    store.create_project(&project(A, "Torts"), 50).unwrap();
    let read = store.get_project(A).unwrap();
    let change = ProjectRename {
        name: "Civil".into(),
        updated_at_ms: 5,
        version: read.version,
    };

    let renamed = store.rename_project(A, &change).unwrap();

    assert_eq!((renamed.name.as_str(), renamed.updated_at_ms), ("Civil", 5));
    assert_eq!(store.get_project(A).unwrap().value, renamed);
    assert_code(store.rename_project(A, &change), ErrorCode::VersionConflict);
    assert_code(store.rename_project(B, &change), ErrorCode::ProjectNotFound);
}

fn deletes_and_then_reports_missing(store: &impl ProjectStore) {
    store.create_project(&project(A, "Torts"), 1).unwrap();

    store.delete_project(A).unwrap();

    assert_code(store.get_project(A), ErrorCode::ProjectNotFound);
    assert_code(store.delete_project(A), ErrorCode::ProjectNotFound);
    store.create_project(&project(B, "Room again"), 1).unwrap();
}
