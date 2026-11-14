//! `ask` and `cancel_ask` (docs/contract.md "Desktop IPC"; phase-2 §4). Pre-stream failures reject
//! the command; after the stream opens, every outcome is an `AskEvent` on the channel and `done`
//! or `error` is sent before the command returns. A cancelled ask sends nothing more; its partial
//! answer is stored with `stopReason: "error"`.

use super::{CommandResult, Shared, blocking};
use crate::asks::AskCancel;
use crate::composition::Composition;
use kristoff_contracts::ask_event::AskEvent;
use kristoff_contracts::requests;
use kristoff_contracts::shapes::{CitationDto, MessageDto, UsageDto};
use kristoff_core::model::Citation;
use kristoff_core::ports::{AnswerSink, CancelSignal, SearchKind};
use kristoff_core::usecases::ask::{self, Question};
use std::sync::Arc;
use tauri::State;
use tauri::ipc::Channel;

/// Sends the use case's pushes as channel events, and stops sending once the ask is cancelled.
/// `emit` returns false when the web side is gone; that cancels the ask.
struct ChannelSink<'a> {
    emit: &'a mut dyn FnMut(AskEvent) -> bool,
    cancel: &'a AskCancel,
    opened: bool,
}

impl ChannelSink<'_> {
    fn send(&mut self, event: AskEvent) {
        if !self.cancel.is_cancelled() && !(self.emit)(event) {
            self.cancel.cancel();
        }
    }
}

impl AnswerSink for ChannelSink<'_> {
    fn open(&mut self) {
        self.opened = true;
    }

    fn text(&mut self, text: &str) {
        self.send(AskEvent::Text {
            text: text.to_owned(),
        });
    }

    fn citation(&mut self, citation: &Citation) {
        self.send(AskEvent::Citation(CitationDto::from(citation)));
    }

    fn search(&mut self, query: &str, count: usize, kind: SearchKind) {
        let query = query.to_owned();
        self.send(AskEvent::Search {
            query,
            count,
            kind: kind.as_str(),
        });
    }

    fn follow_ups(&mut self, questions: &[String]) {
        self.send(AskEvent::Followups {
            follow_ups: questions.to_vec(),
        });
    }
}

/// Answers one question, sending its events through `emit`.
///
/// # Errors
/// Before the stream opens: `VALIDATION_FAILED`, `CHAT_NOT_FOUND`,
/// `DAILY_CAP_REACHED`, `DOC_NOT_FOUND`. After it, failures are `error` events, not errors.
pub fn ask_now(
    app: &Composition,
    chat_id: &str,
    question: (&str, Option<&str>),
    emit: &mut dyn FnMut(AskEvent) -> bool,
) -> CommandResult<()> {
    let chat_id = requests::id(chat_id)?;
    let (text, model) = requests::ask(question.0, question.1)?;
    let cancel: Arc<AskCancel> = app.asks.begin(chat_id);
    let mut sink = ChannelSink {
        emit,
        cancel: &cancel,
        opened: false,
    };
    let outcome = ask::ask(
        &app.ask()?,
        Question {
            chat_id,
            text,
            model,
        },
        &mut sink,
        cancel.as_ref(),
    );
    app.asks.end(chat_id, &cancel);
    match outcome {
        Ok(done) => {
            let message = MessageDto::from(&done.message);
            sink.send(AskEvent::Done {
                message,
                usage: UsageDto::from(&done.usage),
            });
            Ok(())
        }
        Err(e) if sink.opened => {
            sink.send(AskEvent::error(&e, app.request_id()));
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Cancels the running answer of a chat, if any.
///
/// # Errors
/// `VALIDATION_FAILED`.
pub fn cancel(app: &Composition, chat_id: &str) -> CommandResult<()> {
    app.asks.cancel(requests::id(chat_id)?);
    Ok(())
}

/// `ask {chatId, text, model, onEvent}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
// The State plus the four IPC arguments {chatId, text, model, onEvent} are the wire contract.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn ask(
    app: State<'_, Shared>,
    chat_id: String,
    text: String,
    model: Option<String>,
    on_event: Channel<AskEvent>,
) -> CommandResult<()> {
    blocking(&app, move |c| {
        let mut emit = |event: AskEvent| on_event.send(event).is_ok();
        ask_now(c, &chat_id, (&text, model.as_deref()), &mut emit)
    })
    .await
}

/// `cancel_ask {chatId}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn cancel_ask(app: State<'_, Shared>, chat_id: String) -> CommandResult<()> {
    blocking(&app, move |c| cancel(c, &chat_id)).await
}
