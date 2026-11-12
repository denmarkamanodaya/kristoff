//! Shared setup for the ask use-case tests: ids, a recording sink and scripted answers.
#![allow(dead_code)] // each test file uses a different subset

use kristoff_core::domain::model_request::{AnswerEnd, CitationSource, ModelEvent};
use kristoff_core::error::AppError;
use kristoff_core::model::Citation;
use kristoff_core::ports::{AnswerSink, ChatStore, NeverCancelled};
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
        }),
    }
}
