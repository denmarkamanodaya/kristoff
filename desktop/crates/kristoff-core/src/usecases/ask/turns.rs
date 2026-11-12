//! The answer loop (docs/desktop/phases/phase-4-statute-tools.md §2): stream a turn; when it
//! stops for `tool_use`, run the statute tools, append the assistant content verbatim and the tool
//! results, and stream again. At most `MAX_TOOL_CALLS` calls per question; after that the next turn
//! is sent with `tool_choice: none`. Text and citations from every turn form one answer, and the
//! turns' usage is summed.

use super::{Answer, AskDeps};
use crate::domain::citation::map_citation;
use crate::domain::law::tools::{LawToolRun, article_results, search_result, validate};
use crate::domain::model_request::{
    AnswerEnd, AttachedDoc, ModelEvent, ModelRequest, ToolAnswer, ToolAnswerContent, ToolCall,
    ToolRound,
};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::limits::{MAX_CITATIONS, MAX_TOOL_CALLS};
use crate::model::{Citation, Usage};
use crate::ports::{AnswerSink, CancelSignal, LawSearch};

/// Sent for a call made after the cap.
const OVER_CAP: &str = "The statute search limit for this question is reached. Answer with what \
                        you already have.";

/// What every turn so far produced.
#[derive(Default)]
struct Draft {
    text: String,
    citations: Vec<Citation>,
    usage: Usage,
}

fn add_usage(total: &mut Usage, more: Usage) {
    total.input += more.input;
    total.output += more.output;
    total.cache_read += more.cache_read;
    total.cache_write += more.cache_write;
}

fn cancelled() -> AppError {
    AppError::new(ErrorCode::UpstreamUnavailable, "the answer was cancelled")
}

/// Streams one turn into the draft and the sink.
fn stream_turn(
    deps: &AskDeps<'_>,
    request: &ModelRequest,
    (docs, draft): (&[AttachedDoc], &mut Draft),
    (sink, cancel): (&mut dyn AnswerSink, &dyn CancelSignal),
) -> AppResult<AnswerEnd> {
    let end = deps
        .model
        .stream(request, cancel, &mut |event| match event {
            ModelEvent::Text(piece) => {
                draft.text.push_str(&piece);
                sink.text(&piece);
            }
            ModelEvent::Citation(source) if draft.citations.len() < MAX_CITATIONS => {
                let n = u32::try_from(draft.citations.len() + 1).unwrap_or(u32::MAX);
                let citation = map_citation(&source, docs, n);
                draft.text.push_str(" [");
                draft.text.push_str(&n.to_string());
                draft.text.push(']');
                sink.citation(&citation);
                draft.citations.push(citation);
            }
            ModelEvent::Citation(_) => {}
        });
    // A cancel that lands after the last event still ends the answer as a failure.
    if cancel.is_cancelled() && end.is_ok() {
        Err(cancelled())
    } else {
        end
    }
}

/// Runs one validated call against the index.
fn run(law: &dyn LawSearch, run: &LawToolRun, sink: &mut dyn AnswerSink) -> ToolAnswerContent {
    let found = match run {
        LawToolRun::Search {
            query,
            filters,
            limit,
        } => law.search(query, filters, *limit).map(|hits| {
            let results: Vec<_> = hits.iter().filter_map(search_result).collect();
            sink.search(query, results.len());
            results
        }),
        LawToolRun::Article {
            law: target,
            article,
        } => law
            .get_article(*target, article)
            .map(|view| view.as_ref().map(article_results).unwrap_or_default()),
    };
    found.map_or_else(
        |_| ToolAnswerContent::Error("The law index failed; answer without it.".to_owned()),
        ToolAnswerContent::Results,
    )
}

/// Answers every call of a turn; `used` counts calls against the cap (refused ones included).
fn answer_calls(
    law: &dyn LawSearch,
    calls: &[ToolCall],
    used: &mut usize,
    sink: &mut dyn AnswerSink,
) -> Vec<ToolAnswer> {
    calls
        .iter()
        .map(|call| {
            let content = if *used >= MAX_TOOL_CALLS {
                ToolAnswerContent::Error(OVER_CAP.to_owned())
            } else {
                *used += 1;
                validate(&call.input).map_or_else(ToolAnswerContent::Error, |r| run(law, &r, sink))
            };
            ToolAnswer {
                tool_use_id: call.id.clone(),
                content,
            }
        })
        .collect()
}

/// Streams the answer, running statute tools between turns, into one [`Answer`].
pub(super) fn answer(
    deps: &AskDeps<'_>,
    mut request: ModelRequest,
    docs: &[AttachedDoc],
    (sink, cancel): (&mut dyn AnswerSink, &dyn CancelSignal),
) -> Answer {
    let mut draft = Draft::default();
    let mut used = 0;
    let end = loop {
        request.tool_choice_none = request.statutes_on && used >= MAX_TOOL_CALLS;
        let turn = match stream_turn(deps, &request, (docs, &mut draft), (&mut *sink, cancel)) {
            Ok(turn) => turn,
            Err(e) => break Err(e),
        };
        add_usage(&mut draft.usage, turn.usage);
        let law = deps
            .law
            .filter(|_| request.statutes_on && !request.tool_choice_none);
        match law {
            Some(law)
                if turn.stop_reason.as_deref() == Some("tool_use")
                    && !turn.tool_calls.is_empty() =>
            {
                let answers = answer_calls(law, &turn.tool_calls, &mut used, sink);
                let content = turn.assistant_content_json;
                request.tool_rounds.push(ToolRound {
                    assistant_content_json: content,
                    answers,
                });
            }
            _ => {
                break Ok(AnswerEnd {
                    usage: draft.usage,
                    ..turn
                });
            }
        }
    };
    Answer {
        text: draft.text,
        citations: draft.citations,
        end,
        model: request.model,
    }
}
