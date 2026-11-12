//! Shared setup for the ask use-case tests: ids, a recording sink and scripted answers.
#![allow(dead_code)] // each test file uses a different subset

use kristoff_core::domain::model_request::{
    AnswerEnd, CitationSource, ModelEvent, RawSearchInput, ToolCall, ToolInput,
};
use kristoff_core::error::AppError;
use kristoff_core::model::Citation;
use kristoff_core::model::Usage;
use kristoff_core::ports::{AnswerSink, ChatStore, NeverCancelled, SearchKind};
use kristoff_core::usecases::ask::{AskOutcome, Question, ask};
use kristoff_testkit::builders::chat;
use kristoff_testkit::fakes::{CancelFlag, Script};
use kristoff_testkit::world::{ANSWER_USAGE, START_MS, World};

pub const C: &str = "00000000000000c1";
pub const D: &str = "00000000000000d1";
pub const P: &str = "00000000000000d2";

/// Records what the use case pushed; optionally cancels when the first text arrives.
#[derive(Default)]
pub struct Recorder<'a> {
    pub events: Vec<String>,
    pub citations: Vec<Citation>,
    pub cancel_on_text: Option<&'a CancelFlag>,
}

impl AnswerSink for Recorder<'_> {
    fn open(&mut self) {
        self.events.push("open".into());
    }
    fn text(&mut self, text: &str) {
        self.events.push(format!("text:{text}"));
        if let Some(flag) = self.cancel_on_text {
            flag.cancel();
        }
    }
    fn citation(&mut self, citation: &Citation) {
        self.events.push(format!("citation:{}", citation.n));
        self.citations.push(citation.clone());
    }
    fn follow_ups(&mut self, questions: &[String]) {
        self.events.push(format!("followups:{}", questions.len()));
    }
    fn search(&mut self, query: &str, count: usize, kind: SearchKind) {
        self.events
            .push(format!("search:{query}:{count}:{}", kind.as_str()));
    }
}

pub fn world_with_chat(doc_ids: &[&str]) -> World {
    let w = World::new();
    w.store
        .create_chat(&chat(C, START_MS - 1000, doc_ids))
        .unwrap();
    w
}

pub fn q(text: &str) -> Question<'_> {
    Question {
        chat_id: C,
        text,
        model: None,
    }
}

pub fn run(w: &World, text: &str, sink: &mut Recorder<'_>) -> Result<AskOutcome, AppError> {
    ask(&w.ask(), q(text), sink, &NeverCancelled)
}

pub fn page_citation(index: usize, start: u32) -> ModelEvent {
    ModelEvent::Citation(CitationSource::Page {
        document_index: index,
        document_title: None,
        start_page: start,
        end_page_exclusive: start + 1,
        cited_text: "held\r\nthat".into(),
    })
}

pub fn script(events: Vec<ModelEvent>, stop: &str) -> Script {
    Script {
        events,
        end: Ok(AnswerEnd {
            usage: ANSWER_USAGE,
            stop_reason: Some(stop.into()),
            ..AnswerEnd::default()
        }),
    }
}

/// The usage of every scripted tool-loop turn.
pub const TURN: Usage = Usage {
    input: 10,
    output: 5,
    cache_read: 1,
    cache_write: 2,
};

/// A `search_ph_laws` call.
pub fn search_call(id: &str, query: &str) -> ToolCall {
    let raw = RawSearchInput {
        query: Some(query.into()),
        ..RawSearchInput::default()
    };
    ToolCall {
        id: id.into(),
        input: ToolInput::SearchLaws(raw),
    }
}

/// A turn that says `text`, then stops to make `calls`.
pub fn tool_turn(text: &str, calls: Vec<ToolCall>) -> Script {
    Script {
        events: vec![ModelEvent::Text(text.into())],
        end: Ok(AnswerEnd {
            usage: TURN,
            stop_reason: Some("tool_use".into()),
            tool_calls: calls,
            assistant_content_json: format!(
                r#"[{{"type":"thinking","thinking":"","signature":"S"}},{{"type":"text","text":"{text}"}}]"#
            ),
        }),
    }
}

/// A last turn that ends the answer.
pub fn final_turn(events: Vec<ModelEvent>) -> Script {
    Script {
        events,
        end: Ok(AnswerEnd::finished(TURN, "end_turn")),
    }
}
