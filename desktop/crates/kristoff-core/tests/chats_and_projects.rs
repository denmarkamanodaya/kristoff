//! Chat and project use cases: ported from `resolve-attached-docs`, `create-chat`, `update-chat`,
//! `delete-chat`, `create-project`, `list-projects` and `delete-project` tests.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::error::ErrorCode;
use kristoff_core::limits::MAX_PROJECTS;
use kristoff_core::model::DocStatus;
use kristoff_core::ports::{ChatChanges, ChatStore, DocStore, ProjectChange};
use kristoff_core::usecases::chats::{
    create_chat, delete_chat, get_chat, resolve_attached_docs, update_chat,
};
use kristoff_core::usecases::projects::{
    create_project, delete_project, list_projects, rename_project,
};
use kristoff_testkit::builders::{answer, chat, doc, part};
use kristoff_testkit::contract::assert_code;
use kristoff_testkit::world::{START_MS, World};

const READY: &str = "00000000000000d1";
const SPLIT: &str = "00000000000000d2";
const PENDING: &str = "00000000000000d3";

fn library(w: &World) {
    w.store.create_doc(&doc(READY, DocStatus::Ready)).unwrap();
    let mut split = doc(SPLIT, DocStatus::Ready);
    split.parts = vec![part(1), part(2)];
    w.store.create_doc(&split).unwrap();
    w.store
        .create_doc(&doc(PENDING, DocStatus::Pending))
        .unwrap();
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn attaches_ready_docs_and_parts_de_duplicated_in_request_order() {
    let w = World::new();
    library(&w);

    let got = resolve_attached_docs(&w.chats(), &ids(&["00000000000000d2:p2", READY, READY]));

    assert_eq!(got.unwrap(), ids(&["00000000000000d2:p2", READY]));
}

#[test]
fn refuses_unusable_attachments() {
    let w = World::new();
    library(&w);
    for bad in [
        PENDING,
        SPLIT,
        "00000000000000d2:p3",
        "00000000000000ff",
        "nonsense",
    ] {
        assert_code(
            resolve_attached_docs(&w.chats(), &ids(&[bad])),
            ErrorCode::DocNotFound,
        );
    }
}

#[test]
fn refuses_more_attachments_than_the_cap() {
    let w = World::new();
    library(&w);
    let six = ids(&[
        READY,
        "00000000000000d2:p1",
        "00000000000000d2:p2",
        "a",
        "b",
        "c",
    ]);

    assert_code(
        resolve_attached_docs(&w.chats(), &six),
        ErrorCode::ValidationFailed,
    );
}

#[test]
fn creates_a_new_chat_with_its_attachments() {
    let w = World::new();
    library(&w);

    let created = create_chat(&w.chats(), &ids(&[READY])).unwrap();

    assert_eq!(
        (
            created.title.as_str(),
            created.pinned,
            created.project_id.as_deref()
        ),
        ("New chat", false, None)
    );
    assert_eq!(
        (created.created_at_ms, created.updated_at_ms),
        (START_MS, START_MS)
    );
    assert_eq!(w.store.get_chat(&created.id).unwrap().value, created);
}

#[test]
fn updates_title_pin_attachments_and_project() {
    let w = World::new();
    library(&w);
    let c = create_chat(&w.chats(), &[]).unwrap();
    let project = create_project(&w.projects(), "Torts").unwrap();
    w.clock.advance(5);
    let changes = ChatChanges {
        title: Some(" Digest ".into()),
        doc_ids: Some(ids(&[READY])),
        pinned: Some(true),
        project: Some(ProjectChange::MoveTo(project.id.clone())),
    };

    let updated = update_chat(&w.chats(), &c.id, &changes).unwrap();

    assert_eq!((updated.title.as_str(), updated.pinned), ("Digest", true));
    assert_eq!(
        (updated.doc_ids, updated.project_id),
        (ids(&[READY]), Some(project.id))
    );
    assert_eq!(updated.updated_at_ms, START_MS + 5);
}

#[test]
fn refuses_an_update_to_a_missing_project_chat_or_blank_title() {
    let w = World::new();
    let c = create_chat(&w.chats(), &[]).unwrap();
    let to_missing = ChatChanges {
        project: Some(ProjectChange::MoveTo("00000000000000f9".into())),
        ..ChatChanges::default()
    };
    let blank = ChatChanges {
        title: Some("  ".into()),
        ..ChatChanges::default()
    };

    assert_code(
        update_chat(&w.chats(), &c.id, &to_missing),
        ErrorCode::ProjectNotFound,
    );
    assert_code(
        update_chat(&w.chats(), &c.id, &blank),
        ErrorCode::ValidationFailed,
    );
    assert_code(
        update_chat(&w.chats(), "00000000000000c9", &blank),
        ErrorCode::ChatNotFound,
    );
}

#[test]
fn deletes_a_chat_with_its_messages() {
    let w = World::new();
    let c = create_chat(&w.chats(), &[]).unwrap();
    w.store
        .put_message(&c.id, &answer("01KA0000000000000000000001", 1))
        .unwrap();
    assert_eq!(get_chat(&w.chats(), &c.id).unwrap().messages.len(), 1);

    delete_chat(&w.chats(), &c.id).unwrap();

    assert_code(get_chat(&w.chats(), &c.id), ErrorCode::ChatNotFound);
    assert_eq!(w.store.recent_messages(&c.id, 10).unwrap(), vec![]);
}

#[test]
fn refuses_the_project_past_the_cap_of_50() {
    let w = World::new();
    for i in 0..MAX_PROJECTS {
        create_project(&w.projects(), &format!("P{i}")).unwrap();
    }

    assert_code(
        create_project(&w.projects(), "One more"),
        ErrorCode::ProjectLimitReached,
    );
}

#[test]
fn lists_projects_in_name_order_and_renames() {
    let w = World::new();
    let b = create_project(&w.projects(), "Torts").unwrap();
    create_project(&w.projects(), "Crim").unwrap();
    rename_project(&w.projects(), &b.id, "Admin").unwrap();

    let names: Vec<_> = list_projects(&w.projects())
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();

    assert_eq!(names, ["Admin", "Crim"]);
    assert_code(
        rename_project(&w.projects(), &b.id, " "),
        ErrorCode::ValidationFailed,
    );
}

#[test]
fn delete_project_ungroups_its_chats_and_leaves_others() {
    let w = World::new();
    let p = create_project(&w.projects(), "Torts").unwrap();
    let mut grouped = chat("00000000000000c1", 10, &[]);
    grouped.project_id = Some(p.id.clone());
    w.store.create_chat(&grouped).unwrap();
    let mut elsewhere = chat("00000000000000c2", 20, &[]);
    elsewhere.project_id = Some("00000000000000f9".into());
    w.store.create_chat(&elsewhere).unwrap();

    delete_project(&w.projects(), &p.id).unwrap();

    assert_eq!(
        w.store
            .get_chat("00000000000000c1")
            .unwrap()
            .value
            .project_id,
        None
    );
    let kept = w
        .store
        .get_chat("00000000000000c2")
        .unwrap()
        .value
        .project_id;
    assert_eq!(kept.as_deref(), Some("00000000000000f9"));
    assert_code(
        delete_project(&w.projects(), &p.id),
        ErrorCode::ProjectNotFound,
    );
}
