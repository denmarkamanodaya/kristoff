//! How an answer ends: cut off at the length limit, failed, or cancelled (also a cancel that loses
//! the race with the stream's end). The answer is ALWAYS stored.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod ask_support;

use ask_support::*;
use kristoff_core::domain::model_request::{AnswerEnd, FollowUps, ModelEvent, ModelRequest};
use kristoff_core::error::{AppError, ErrorCode};
use kristoff_core::model::Usage;
use kristoff_core::ports::{AnswerModel, CancelSignal, ChatStore, UsageMeter};
use kristoff_core::usecases::ask::{CUT_OFF_NOTE, FAILED_NOTE, ask};
use kristoff_core::util::time::utc_day;
use kristoff_testkit::contract::assert_code;
use kristoff_testkit::fakes::{CancelFlag, FixedFollowUps, Script, ScriptedModel};
use kristoff_testkit::world::{ANSWER_USAGE, START_MS};

#[test]
fn an_answer_cut_at_max_tokens_gets_the_cut_off_note() {
    let mut w = world_with_chat(&[]);
    w.model = ScriptedModel::new(script(vec![ModelEvent::Text("Long".into())], "max_tokens"));

    let done = run(&w, "q", &mut Recorder::default()).unwrap();

    assert_eq!(done.message.text, format!("Long{CUT_OFF_NOTE}"));
    assert_eq!(done.message.stop_reason.as_deref(), Some("max_tokens"));
}

#[test]
fn a_failed_answer_is_stored_with_the_failed_note_and_no_follow_ups_or_tokens() {
    let mut w = world_with_chat(&[]);
    let failure = AppError::new(ErrorCode::UpstreamUnavailable, "overloaded");
    w.model = ScriptedModel::new(Script {
        events: vec![ModelEvent::Text("Part".into())],
        end: Err(failure),
    });
    w.follow_ups = FixedFollowUps::returning(FollowUps {
        questions: vec!["x".into()],
        usage: None,
    });
    let mut sink = Recorder::default();

    assert_code(run(&w, "q", &mut sink), ErrorCode::UpstreamUnavailable);

    let answer = w.store.recent_messages(C, 10).unwrap().pop().unwrap();
    assert_eq!(answer.text, format!("Part{FAILED_NOTE}"));
    assert_eq!(
        (answer.stop_reason.as_deref(), answer.usage),
        (Some("error"), None)
    );
    assert_eq!(w.follow_ups.asked(), vec![]);
    assert_eq!(sink.events, ["open", "text:Part"]);
    assert_eq!(
        w.store.read_usage(&utc_day(START_MS)).unwrap().today,
        Usage::default()
    );
}

#[test]
fn a_cancelled_answer_keeps_its_partial_text_with_stop_reason_error() {
    let mut w = world_with_chat(&[]);
    w.model = ScriptedModel::new(script(
        vec![
            ModelEvent::Text("Partial".into()),
            ModelEvent::Text(" more".into()),
        ],
        "end_turn",
    ));
    let cancel = CancelFlag::default();
    let mut sink = Recorder {
        cancel_on_text: Some(&cancel),
        ..Recorder::default()
    };

    let result = ask(&w.ask(), q("q"), &mut sink, &cancel);

    assert_code(result, ErrorCode::UpstreamUnavailable);
    assert!(cancel.is_cancelled());
    let answer = w.store.recent_messages(C, 10).unwrap().pop().unwrap();
    assert_eq!(answer.text, format!("Partial{FAILED_NOTE}"));
    assert_eq!(answer.stop_reason.as_deref(), Some("error"));
    assert_eq!(sink.events, ["open", "text:Partial"]);
    assert_eq!(w.follow_ups.asked(), vec![]);
}

/// A model whose stream finished before it saw the cancel (the race a real stream can lose):
/// it streams one piece, the cancel lands, and it still returns `end_turn`.
struct FinishesDespiteCancel;

impl AnswerModel for FinishesDespiteCancel {
    fn stream(
        &self,
        _request: &ModelRequest,
        _cancel: &dyn CancelSignal,
        on_event: &mut dyn FnMut(ModelEvent),
    ) -> Result<AnswerEnd, AppError> {
        on_event(ModelEvent::Text("Done".into()));
        Ok(AnswerEnd {
            usage: ANSWER_USAGE,
            stop_reason: Some("end_turn".into()),
            ..AnswerEnd::default()
        })
    }
}

#[test]
fn a_cancel_that_lands_after_the_last_event_still_stores_a_failed_answer_without_follow_ups() {
    let mut w = world_with_chat(&[]);
    w.follow_ups = FixedFollowUps::returning(FollowUps {
        questions: vec!["x".into()],
        usage: None,
    });
    let cancel = CancelFlag::default();
    let mut sink = Recorder {
        cancel_on_text: Some(&cancel),
        ..Recorder::default()
    };
    let mut deps = w.ask();
    deps.model = &FinishesDespiteCancel;

    assert_code(
        ask(&deps, q("q"), &mut sink, &cancel),
        ErrorCode::UpstreamUnavailable,
    );

    let answer = w.store.recent_messages(C, 10).unwrap().pop().unwrap();
    assert_eq!(answer.text, format!("Done{FAILED_NOTE}"));
    assert_eq!(answer.stop_reason.as_deref(), Some("error"));
    assert_eq!(w.follow_ups.asked(), vec![]);
    assert_eq!(sink.events, ["open", "text:Done"]);
}
