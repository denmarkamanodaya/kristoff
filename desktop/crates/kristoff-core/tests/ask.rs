//! The ask use case: ported from `ask-question/v1.test.ts`, plus file ids, cancel and the desktop
//! attachment rule.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kristoff_core::domain::model_request::{FollowUps, ModelEvent, TurnBlock};
use kristoff_core::error::ErrorCode;
use kristoff_core::model::DocStatus;
use kristoff_core::model::{Role, Usage};
use kristoff_core::ports::{ChatStore, DocStore, NeverCancelled, UsageMeter};
use kristoff_core::usecases::ask::{Question, ask};
use kristoff_core::util::time::utc_day;
use kristoff_testkit::builders::{doc, part, uploaded_doc};
use kristoff_testkit::contract::assert_code;
use kristoff_testkit::fakes::{FixedFollowUps, ScriptedModel};
use kristoff_testkit::world::{ANSWER_USAGE, START_MS, World};

mod ask_support;

use ask_support::*;

/// A world whose chat attaches one uploaded doc; the model answers "Held [1]." and Haiku
/// suggests "Why?" for 9 + 3 tokens.
fn answering_world() -> World {
    let mut w = world_with_chat(&[D]);
    w.store.create_doc(&uploaded_doc(D)).unwrap();
    let events = vec![
        ModelEvent::Text("Held".into()),
        page_citation(0, 3),
        ModelEvent::Text(".".into()),
    ];
    w.model = ScriptedModel::new(script(events, "end_turn"));
    let usage = Usage {
        input: 9,
        output: 3,
        cache_read: 0,
        cache_write: 0,
    };
    w.follow_ups = FixedFollowUps::returning(FollowUps {
        questions: vec!["Why?".into()],
        usage: Some(usage),
    });
    w
}

#[test]
fn streams_the_answer_and_stores_both_messages() {
    let w = answering_world();
    let mut sink = Recorder::default();

    let done = run(&w, "What was held?", &mut sink).unwrap();

    assert_eq!(
        sink.events,
        ["open", "text:Held", "citation:1", "text:.", "followups:1"]
    );
    assert_eq!(done.message.text, "Held [1].");
    assert_eq!(
        (done.message.stop_reason.as_deref(), done.usage),
        (Some("end_turn"), ANSWER_USAGE)
    );
    assert_eq!(done.message.follow_ups, ["Why?"]);
    assert_eq!(sink.citations[0].quote, "held\nthat");
    let stored = w.store.recent_messages(C, 10).unwrap();
    let roles: Vec<Role> = stored.iter().map(|m| m.role).collect();
    assert_eq!(roles, [Role::User, Role::Assistant]);
    assert_eq!(stored[1], done.message);
}

#[test]
fn counts_the_answer_and_follow_up_tokens_for_today() {
    let w = answering_world();

    run(&w, "What was held?", &mut Recorder::default()).unwrap();

    let today = w.store.read_usage(&utc_day(START_MS)).unwrap().today;
    assert_eq!((today.input, today.output), (109, 23));
    let asked = w.follow_ups.asked();
    assert_eq!(
        asked,
        [("What was held?".to_owned(), "Held [1].".to_owned())]
    );
}

#[test]
fn uses_the_default_model_and_refuses_one_not_allowed_before_counting() {
    let w = world_with_chat(&[]);

    run(&w, "q", &mut Recorder::default()).unwrap();
    let refused = ask(
        &w.ask(),
        Question {
            chat_id: C,
            text: "q",
            model: Some("claude-opus-5-5"),
        },
        &mut Recorder::default(),
        &NeverCancelled,
    );

    assert_eq!(w.model.requests()[0].model, "claude-sonnet-5-5");
    assert_code(refused, ErrorCode::ValidationFailed);
    assert_eq!(w.model.requests().len(), 1);
    assert_eq!(w.store.recent_messages(C, 10).unwrap().len(), 2);
}

#[test]
fn refuses_an_unknown_chat() {
    let w = World::new();

    assert_code(
        run(&w, "q", &mut Recorder::default()),
        ErrorCode::ChatNotFound,
    );
}

#[test]
fn refuses_at_the_daily_cap_with_the_wait_until_midnight_utc() {
    let mut w = world_with_chat(&[]);
    w.daily_cap = 1;
    run(&w, "first", &mut Recorder::default()).unwrap();

    let err = run(&w, "second", &mut Recorder::default()).unwrap_err();

    assert_eq!(err.code, ErrorCode::DailyCapReached);
    // START_MS is 01:00:00 UTC, so 23 hours remain.
    assert_eq!(err.retry_after_seconds, Some(23 * 3600));
    assert_eq!(w.model.requests().len(), 1);
}

#[test]
fn refuses_with_key_missing_when_no_key_is_saved_and_counts_nothing() {
    let mut w = world_with_chat(&[]);
    w.key_configured = false;
    w.daily_cap = 1;

    assert_code(
        run(&w, "q", &mut Recorder::default()),
        ErrorCode::KeyMissing,
    );

    assert_eq!(w.model.requests().len(), 0);
    assert_eq!(w.store.recent_messages(C, 10).unwrap().len(), 0);
    // The refused question used none of today's one allowed question.
    w.key_configured = true;
    run(&w, "q", &mut Recorder::default()).unwrap();
}

#[test]
fn refuses_an_attachment_that_is_not_ready_or_not_uploaded() {
    for attached in [doc(D, DocStatus::Processing), doc(D, DocStatus::Ready)] {
        let w = world_with_chat(&[D]);
        w.store.create_doc(&attached).unwrap();
        let mut sink = Recorder::default();

        assert_code(run(&w, "q", &mut sink), ErrorCode::DocNotFound);

        assert_eq!(w.model.requests().len(), 0);
        assert_eq!(sink.events, Vec::<String>::new());
        assert_eq!(w.store.recent_messages(C, 10).unwrap().len(), 0);
    }
}

#[test]
fn a_question_refused_for_its_attachment_is_not_counted_against_the_cap() {
    let w = world_with_chat(&[D]);
    w.store.create_doc(&doc(D, DocStatus::Processing)).unwrap();

    assert_code(
        run(&w, "q", &mut Recorder::default()),
        ErrorCode::DocNotFound,
    );

    // With a cap of 1, the day still has its one question left.
    w.store.admit_question(&utc_day(START_MS), 1).unwrap();
}

#[test]
fn sends_parts_with_page_titles_and_maps_their_citations_back() {
    let p2 = format!("{P}:p2");
    let mut w = world_with_chat(&[D, &p2]);
    w.store.create_doc(&uploaded_doc(D)).unwrap();
    let mut split = uploaded_doc(P);
    (split.file_id, split.parts, split.title) = (None, vec![part(1), part(2)], "Big".into());
    w.store.create_doc(&split).unwrap();
    w.model = ScriptedModel::new(script(vec![page_citation(1, 3)], "end_turn"));
    let mut sink = Recorder::default();

    run(&w, "q", &mut sink).unwrap();

    let blocks = &w.model.requests()[0].turns[0].blocks;
    assert_eq!(
        blocks[0],
        TurnBlock::Doc {
            file_id: format!("file_{D}"),
            title: format!("Doc {D}"),
            is_cache_breakpoint: false
        }
    );
    assert_eq!(
        blocks[1],
        TurnBlock::Doc {
            file_id: "file_part_2".into(),
            title: "Big (pp. 101\u{2013}200)".into(),
            is_cache_breakpoint: true
        }
    );
    let c = &sink.citations[0];
    assert_eq!(
        (
            c.doc_id.as_deref(),
            c.doc_title.as_str(),
            c.page,
            c.end_page
        ),
        (Some(P), "Big", Some(103), Some(103))
    );
}

#[test]
fn titles_the_chat_from_its_first_question_only_and_sends_history() {
    let w = world_with_chat(&[]);
    let long = "x".repeat(70);

    run(&w, &format!("  {long}  "), &mut Recorder::default()).unwrap();
    run(&w, "second", &mut Recorder::default()).unwrap();

    assert_eq!(w.store.get_chat(C).unwrap().value.title, "x".repeat(60));
    let turns = &w.model.requests()[1].turns;
    let texts: Vec<_> = turns.iter().map(|t| (t.role, t.blocks.clone())).collect();
    assert_eq!(texts.len(), 3);
    assert_eq!(
        texts[1],
        (Role::Assistant, vec![TurnBlock::Text("Answer.".into())])
    );
    assert_eq!(
        texts[2],
        (Role::User, vec![TurnBlock::Text("second".into())])
    );
}
