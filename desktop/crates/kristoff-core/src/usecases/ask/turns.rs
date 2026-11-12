//! The answer loop (docs/desktop/phases/phase-4-statute-tools.md §2, phase-7 "Tools"): stream a
//! turn; when it stops for `tool_use`, run the statute and case tools, append the assistant content
//! verbatim and the tool results, and stream again. At most `MAX_TOOL_CALLS` calls per question
//! across all four tools; after that the next turn is sent with `tool_choice: none`. Text and
//! citations from every turn form one answer, and the turns' usage is summed.

use super::{Answer, AskDeps};
use crate::domain::case_tools::{CaseToolRun, NO_CASES_TEXT, case_result, view_result};
use crate::domain::citation::map_citation;
use crate::domain::law::tools::{LawToolRun, NO_RESULTS_TEXT, article_results, search_result};
use crate::domain::model_request::{
    AnswerEnd, AttachedDoc, ModelEvent, ModelRequest, SearchResult, ToolAnswer, ToolAnswerContent,
    ToolCall, ToolRound,
};
use crate::domain::tools::{ToolRun, validate};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::limits::{MAX_CITATIONS, MAX_TOOL_CALLS};
use crate::model::{Citation, CitationKind, Usage};
use crate::ports::{AnswerSink, CancelSignal, CaseSearch, LawSearch, SearchKind};

/// Sent for a call made after the cap.
const OVER_CAP: &str = "The search limit for this question is reached. Answer with what you \
                        already have.";

/// What every turn so far produced.
#[derive(Default)]
struct Draft {
    text: String,
    citations: Vec<Citation>,
    usage: Usage,
    /// The kind (law or case) of every `search_result` sent so far, in `search_result_index` order.
    results: Vec<CitationKind>,
}

/// The indexes this question may search (a tool family that is not offered has none).
#[derive(Clone, Copy)]
struct Indexes<'a> {
    law: Option<&'a dyn LawSearch>,
    cases: Option<&'a dyn CaseSearch>,
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
                let citation = map_citation(&source, docs, &draft.results, n);
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

fn failed(index: &str) -> ToolAnswerContent {
    ToolAnswerContent::Error(format!("The {index} index failed; answer without it."))
}

fn unavailable(family: &str) -> ToolAnswerContent {
    ToolAnswerContent::Error(format!(
        "{family} search is not available for this question."
    ))
}

/// Runs one validated statute call.
fn run_law(law: &dyn LawSearch, run: &LawToolRun, sink: &mut dyn AnswerSink) -> ToolAnswerContent {
    let found = match run {
        LawToolRun::Search {
            query,
            filters,
            limit,
        } => law.search(query, filters, *limit).map(|hits| {
            let results: Vec<_> = hits.iter().filter_map(search_result).collect();
            sink.search(query, results.len(), SearchKind::Law);
            results
        }),
        LawToolRun::Article {
            law: target,
            article,
        } => law
            .get_article(*target, article)
            .map(|view| view.as_ref().map(article_results).unwrap_or_default()),
    };
    found.map_or_else(|_| failed("law"), |r| results_or(r, NO_RESULTS_TEXT))
}

/// The results, or `nothing` as a plain text answer when there are none.
fn results_or(results: Vec<SearchResult>, nothing: &'static str) -> ToolAnswerContent {
    if results.is_empty() {
        ToolAnswerContent::Nothing(nothing)
    } else {
        ToolAnswerContent::Results(results)
    }
}

/// Runs one validated case call.
fn run_case(
    cases: &dyn CaseSearch,
    run: &CaseToolRun,
    sink: &mut dyn AnswerSink,
) -> ToolAnswerContent {
    let found = match run {
        CaseToolRun::Search {
            query,
            filters,
            limit,
        } => cases.search(query, filters, *limit).map(|hits| {
            let results: Vec<_> = hits
                .iter()
                .filter_map(|h| case_result(h, &h.excerpt))
                .collect();
            sink.search(query, results.len(), SearchKind::Case);
            results
        }),
        CaseToolRun::Get { case, focus } => cases
            .get_case(case, focus.as_deref())
            .map(|view| view.as_ref().and_then(view_result).into_iter().collect()),
    };
    found.map_or_else(|_| failed("case"), |r| results_or(r, NO_CASES_TEXT))
}

/// Runs one call and records the kind of every result it sends.
fn run(
    idx: Indexes<'_>,
    call: &ToolCall,
    (sink, draft): (&mut dyn AnswerSink, &mut Draft),
) -> ToolAnswerContent {
    let (content, kind) = match validate(&call.input) {
        Err(message) => (ToolAnswerContent::Error(message), CitationKind::Law),
        Ok(ToolRun::Law(r)) => (
            idx.law
                .map_or_else(|| unavailable("Statute"), |l| run_law(l, &r, sink)),
            CitationKind::Law,
        ),
        Ok(ToolRun::Case(r)) => (
            idx.cases
                .map_or_else(|| unavailable("Case"), |c| run_case(c, &r, sink)),
            CitationKind::Case,
        ),
    };
    if let ToolAnswerContent::Results(results) = &content {
        draft
            .results
            .extend(std::iter::repeat_n(kind, results.len()));
    }
    content
}

/// Answers every call of a turn; `used` counts calls against the cap (refused ones included).
fn answer_calls(
    idx: Indexes<'_>,
    calls: &[ToolCall],
    used: &mut usize,
    (sink, draft): (&mut dyn AnswerSink, &mut Draft),
) -> Vec<ToolAnswer> {
    calls
        .iter()
        .map(|call| {
            let content = if *used >= MAX_TOOL_CALLS {
                ToolAnswerContent::Error(OVER_CAP.to_owned())
            } else {
                *used += 1;
                run(idx, call, (&mut *sink, &mut *draft))
            };
            ToolAnswer {
                tool_use_id: call.id.clone(),
                content,
            }
        })
        .collect()
}

/// Streams the answer, running the tools between turns, into one [`Answer`].
pub(super) fn answer(
    deps: &AskDeps<'_>,
    mut request: ModelRequest,
    docs: &[AttachedDoc],
    (sink, cancel): (&mut dyn AnswerSink, &dyn CancelSignal),
) -> Answer {
    let idx = Indexes {
        law: deps.law.filter(|_| request.statutes_on),
        cases: deps.cases.filter(|_| request.cases_on),
    };
    let tools_on = idx.law.is_some() || idx.cases.is_some();
    let mut draft = Draft::default();
    let mut used = 0;
    let end = loop {
        request.tool_choice_none = tools_on && used >= MAX_TOOL_CALLS;
        let turn = match stream_turn(deps, &request, (docs, &mut draft), (&mut *sink, cancel)) {
            Ok(turn) => turn,
            Err(e) => break Err(e),
        };
        add_usage(&mut draft.usage, turn.usage);
        let wants_tools =
            turn.stop_reason.as_deref() == Some("tool_use") && !turn.tool_calls.is_empty();
        if !(tools_on && !request.tool_choice_none && wants_tools) {
            break Ok(AnswerEnd {
                usage: draft.usage,
                ..turn
            });
        }
        let answers = answer_calls(idx, &turn.tool_calls, &mut used, (&mut *sink, &mut draft));
        let content = turn.assistant_content_json;
        request.tool_rounds.push(ToolRound {
            assistant_content_json: content,
            answers,
        });
    };
    Answer {
        text: draft.text,
        citations: draft.citations,
        end,
        model: request.model,
    }
}
