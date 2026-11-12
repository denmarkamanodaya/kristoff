//! `ChatStore` contract.

use super::assert_code;
use crate::builders::{answer, chat, question};
use kristoff_core::error::ErrorCode;
use kristoff_core::model::{Chat, Page};
use kristoff_core::ports::{ChatChanges, ChatPatch, ChatStore, ProjectChange};

const A: &str = "0000000000000001";
const B: &str = "0000000000000002";
const C: &str = "0000000000000003";
const DOC: &str = "00000000000000d1";
const PROJECT: &str = "00000000000000f1";

/// Runs every `ChatStore` promise against fresh stores from `make`. Moving a chat into a project
/// needs that project to exist in stores that check it, so `make` may seed one with id
/// `00000000000000f1`.
pub fn chat_store_contract<S: ChatStore>(make: impl Fn() -> S) {
    creates_and_reads(&make());
    lists_newest_update_first_in_pages(&make());
    updates_at_the_read_version_only(&make());
    ungroups_and_keeps_unchanged_fields(&make());
    keeps_messages_in_order(&make());
    deletes_messages_and_chats_repeatably(&make());
    clears_a_project_only_when_it_still_matches(&make());
}

fn ids(page: &Page<Chat>) -> Vec<&str> {
    page.items.iter().map(|c| c.id.as_str()).collect()
}

fn creates_and_reads(store: &impl ChatStore) {
    let c = chat(A, 10, &[DOC, "00000000000000d2:p3"]);
    store.create_chat(&c).unwrap();

    assert_eq!(store.get_chat(A).unwrap().value, c);
    assert_code(store.create_chat(&c), ErrorCode::Internal);
    assert_code(store.get_chat(B), ErrorCode::ChatNotFound);
}

fn lists_newest_update_first_in_pages(store: &impl ChatStore) {
    store.create_chat(&chat(A, 30, &[])).unwrap();
    store.create_chat(&chat(B, 10, &[])).unwrap();
    store.create_chat(&chat(C, 30, &[])).unwrap();

    let first = store.list_chats(None, 2).unwrap();
    let second = store.list_chats(first.next_cursor.as_deref(), 2).unwrap();

    assert_eq!(ids(&first), [C, A]);
    assert_eq!(ids(&second), [B]);
    assert_eq!(second.next_cursor, None);
    assert_code(store.list_chats(Some("!!"), 2), ErrorCode::ValidationFailed);
}

fn updates_at_the_read_version_only(store: &impl ChatStore) {
    store.create_chat(&chat(A, 10, &[DOC])).unwrap();
    let read = store.get_chat(A).unwrap();
    let changes = ChatChanges {
        title: Some("Torts".into()),
        doc_ids: Some(vec!["00000000000000d2".into(), DOC.into()]),
        pinned: Some(true),
        project: Some(ProjectChange::MoveTo(PROJECT.into())),
        statutes: Some(false),
    };
    let patch = ChatPatch {
        changes,
        updated_at_ms: 99,
    };

    let updated = store.update_chat(A, &patch, read.version).unwrap();

    assert_eq!(
        (
            updated.title.as_str(),
            updated.pinned,
            updated.updated_at_ms
        ),
        ("Torts", true, 99)
    );
    assert_eq!(updated.doc_ids, ["00000000000000d2", DOC]);
    assert_eq!(updated.project_id.as_deref(), Some(PROJECT));
    assert!(!updated.statutes);
    assert_eq!(store.get_chat(A).unwrap().value, updated);
    assert_code(
        store.update_chat(A, &patch, read.version),
        ErrorCode::VersionConflict,
    );
    assert_code(store.update_chat(B, &patch, 1), ErrorCode::ChatNotFound);
}

fn ungroups_and_keeps_unchanged_fields(store: &impl ChatStore) {
    let mut grouped = chat(A, 10, &[DOC]);
    grouped.project_id = Some(PROJECT.into());
    grouped.title = "Torts".into();
    store.create_chat(&grouped).unwrap();
    let changes = ChatChanges {
        project: Some(ProjectChange::Ungroup),
        ..ChatChanges::default()
    };
    let ungroup = ChatPatch {
        changes,
        updated_at_ms: 100,
    };

    let ungrouped = store
        .update_chat(A, &ungroup, store.get_chat(A).unwrap().version)
        .unwrap();

    assert_eq!(
        (ungrouped.project_id, ungrouped.title.as_str()),
        (None, "Torts")
    );
    assert_eq!(
        (ungrouped.doc_ids, ungrouped.updated_at_ms),
        (vec![DOC.to_owned()], 100)
    );
}

fn keeps_messages_in_order(store: &impl ChatStore) {
    store.create_chat(&chat(A, 10, &[])).unwrap();
    let q = question("01KA0000000000000000000001", 1);
    let a = answer("01KA0000000000000000000002", 2);
    let later = question("01KA0000000000000000000003", 3);
    for m in [&a, &q, &later] {
        store.put_message(A, m).unwrap();
    }

    assert_eq!(
        store.recent_messages(A, 10).unwrap(),
        [q.clone(), a.clone(), later.clone()]
    );
    assert_eq!(store.recent_messages(A, 2).unwrap(), [a.clone(), later]);
    assert_code(store.put_message(A, &q), ErrorCode::Internal);
    assert_code(store.put_message(B, &a), ErrorCode::ChatNotFound);
}

fn deletes_messages_and_chats_repeatably(store: &impl ChatStore) {
    store.create_chat(&chat(A, 10, &[])).unwrap();
    store
        .put_message(A, &question("01KA0000000000000000000001", 1))
        .unwrap();
    store
        .put_message(A, &answer("01KA0000000000000000000002", 2))
        .unwrap();

    assert_eq!(store.delete_messages(A).unwrap(), 2);
    assert_eq!(store.delete_messages(A).unwrap(), 0);
    store.delete_chat(A).unwrap();
    store.delete_chat(A).unwrap();
    assert_code(store.get_chat(A), ErrorCode::ChatNotFound);
}

fn clears_a_project_only_when_it_still_matches(store: &impl ChatStore) {
    let mut grouped = chat(A, 10, &[]);
    grouped.project_id = Some(PROJECT.into());
    store.create_chat(&grouped).unwrap();
    store.create_chat(&chat(B, 20, &[])).unwrap();

    store.clear_project(A, "00000000000000f9").unwrap();
    assert_eq!(
        store.get_chat(A).unwrap().value.project_id.as_deref(),
        Some(PROJECT)
    );
    store.clear_project(A, PROJECT).unwrap();
    store.clear_project(B, PROJECT).unwrap();

    let cleared = store.get_chat(A).unwrap().value;
    assert_eq!((cleared.project_id, cleared.updated_at_ms), (None, 10));
}
