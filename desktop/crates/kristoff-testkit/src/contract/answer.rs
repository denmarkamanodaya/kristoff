//! `AnswerModel` and `FollowUpSuggester` contracts.

use super::assert_code;
use crate::fakes::{CancelFlag, Script};
use kristoff_core::domain::model_request::{
    AnswerEnd, CitationSource, ModelEvent, ModelRequest, ModelTurn, TurnBlock,
};
use kristoff_core::error::ErrorCode;
use kristoff_core::model::{Role, Usage};
use kristoff_core::ports::{AnswerModel, FollowUpSuggester, NeverCancelled};

/// A small request with one document, as the ask use case builds it.
#[must_use]
pub fn sample_request() -> ModelRequest {
    let doc = TurnBlock::Doc {
        file_id: "file_011ABC".into(),
        title: "People v. Cruz".into(),
        is_cache_breakpoint: true,
    };
    ModelRequest {
        model: "claude-sonnet-5-5".into(),
        max_tokens: 64,
        turns: vec![ModelTurn {
            role: Role::User,
            blocks: vec![doc, TurnBlock::Text("q".into())],
        }],
        statutes_on: false,
        tool_rounds: vec![],
    }
}

/// The script every model in the contract must play back.
#[must_use]
pub fn sample_script(stop_reason: &str) -> Script {
    let citation = CitationSource::Page {
        document_index: 0,
        document_title: Some("People v. Cruz".into()),
        start_page: 3,
        end_page_exclusive: 4,
        cited_text: "The Court holds \u{2026}".into(),
    };
    Script {
        events: vec![
            ModelEvent::Text("The Court ".into()),
            ModelEvent::Citation(citation),
            ModelEvent::Text("ruled.".into()),
        ],
        end: Ok(AnswerEnd {
            usage: Usage {
                input: 26,
                output: 160,
                cache_read: 0,
                cache_write: 10_214,
            },
            stop_reason: Some(stop_reason.into()),
        }),
    }
}

/// Runs every `AnswerModel` promise. `make` builds a model that answers any request with the
/// given script (the real adapter: a local server streaming it as SSE).
pub fn answer_model_contract<M: AnswerModel>(make: impl Fn(&Script) -> M) {
    plays_the_answer_in_order(&make);
    keeps_a_max_tokens_stop(&make);
    refuses_when_already_cancelled(&make);
}

fn plays_the_answer_in_order<M: AnswerModel>(make: &impl Fn(&Script) -> M) {
    let script = sample_script("end_turn");
    let mut seen = Vec::new();

    let end = make(&script).stream(&sample_request(), &NeverCancelled, &mut |e| seen.push(e));

    assert_eq!(seen, script.events);
    assert_eq!(end, script.end);
}

fn keeps_a_max_tokens_stop<M: AnswerModel>(make: &impl Fn(&Script) -> M) {
    let script = sample_script("max_tokens");

    let end = make(&script)
        .stream(&sample_request(), &NeverCancelled, &mut |_| {})
        .unwrap();

    assert_eq!(end.stop_reason.as_deref(), Some("max_tokens"));
}

fn refuses_when_already_cancelled<M: AnswerModel>(make: &impl Fn(&Script) -> M) {
    let cancel = CancelFlag::default();
    cancel.cancel();
    let mut seen = Vec::new();

    let end =
        make(&sample_script("end_turn")).stream(&sample_request(), &cancel, &mut |e| seen.push(e));

    assert_code(end, ErrorCode::UpstreamUnavailable);
    assert_eq!(seen, vec![]);
}

/// What the follow-up model replies in one contract case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowUpReply {
    /// A valid structured reply with these questions, ending `end_turn`.
    Valid(Vec<String>),
    /// The same questions, but the reply stopped at `max_tokens`.
    CutOff(Vec<String>),
    /// A reply whose text is not the schema's JSON.
    Garbled,
}

/// Runs every `FollowUpSuggester` promise. `make` builds a suggester whose model replies `reply`.
///
/// # Panics
/// On the first broken promise.
pub fn follow_up_suggester_contract<F: FollowUpSuggester>(make: impl Fn(FollowUpReply) -> F) {
    let questions = vec!["What is BWS?".to_owned(), "Is it a defense?".to_owned()];

    let valid = make(FollowUpReply::Valid(questions.clone())).suggest("q", "a");
    let cut = make(FollowUpReply::CutOff(questions.clone())).suggest("q", "a");
    let garbled = make(FollowUpReply::Garbled).suggest("q", "a");

    assert_eq!(valid.questions, questions);
    assert!(valid.usage.is_some());
    assert_eq!(
        cut.questions,
        Vec::<String>::new(),
        "trusted only on end_turn"
    );
    assert_eq!(garbled.questions, Vec::<String>::new());
}
