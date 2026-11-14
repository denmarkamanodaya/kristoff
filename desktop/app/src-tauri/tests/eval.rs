//! The Phase 5 eval runner (docs/desktop/phases/phase-5-eval-package.md §1), IGNORED by default:
//! it sends every question in `contracts/fixtures/eval/questions.json` through the real ask use
//! case (real `LawIndex`, Sonnet 5.5, a temp data dir) and writes
//! `desktop/target/eval/report-<date>.json`. It costs about US$1.40 and stops near US$2.
//!
//! Run by hand, after `node desktop/scripts/eval-docs-to-pdf.mjs` and with the real key baked:
//! `cargo test -p kristoff-app --test eval -- --ignored --nocapture`
//! (`KRISTOFF_PHLAWS_DB` / `KRISTOFF_E5_MODEL_DIR` default to `desktop/data`).

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout
)]

mod eval_support;
mod support;

use eval_support::claims::{Cite, uncited_statutory_sentences};
use eval_support::questions::{Question, load};
use eval_support::recorder::{Recorded, RecordingLaw};
use eval_support::score::{HAIKU, SONNET, cost, doc_page_cited, summary, target_hits};
use kristoff_app::commands::upload::RawChunk;
use kristoff_app::commands::{ask, chats, docs, session, upload};
use kristoff_app::composition::Composition;
use kristoff_app::law::{DB_ENV, LawPaths, MODEL_ENV, open_law_index};
use kristoff_core::model::Usage;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// The real composition on `dir` with the Claude key given at run time (`KRISTOFF_LIVE_KEY`),
/// kept in memory instead of the Keychain.
fn live_composition(dir: &std::path::Path) -> Composition {
    let key = std::env::var("KRISTOFF_LIVE_KEY").expect("KRISTOFF_LIVE_KEY");
    let ai =
        kristoff_app::ai::AiPorts::anthropic(kristoff_anthropic::SharedKey::from(key.as_str()))
            .unwrap();
    let secrets = Box::new(kristoff_testkit::fakes::MemorySecretStore::holding(&key));
    let clock = Box::new(kristoff_adapters::system::SystemClock);
    Composition::with_ports(dir, clock, ai, secrets).unwrap()
}

/// The run stops before its total cost passes this (lead's ceiling for run 2).
const BUDGET_USD: f64 = 2.0;
const MODEL: &str = "claude-sonnet-5-5";

fn desktop_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn open_app(dir: &Path, log: &Recorded) -> Composition {
    let core = live_composition(dir);
    let data = desktop_dir().join("data");
    let env_or = |key: &str, default: PathBuf| std::env::var_os(key).map_or(default, PathBuf::from);
    let paths = LawPaths {
        db: env_or(DB_ENV, data.join("phlaws.db")),
        cases_db: data.join("phcases.db"),
        model_dir: env_or(MODEL_ENV, data.join("model")),
    };
    let inner = open_law_index(&paths).unwrap();
    core.install_law(Box::new(RecordingLaw {
        inner,
        log: log.clone(),
    }));
    core
}

fn upload_pdf(core: &Composition, name: &str) -> String {
    let path = desktop_dir()
        .join("target/eval/pdfs")
        .join(format!("{name}.pdf"));
    let pdf = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("{}: {e} (run eval-docs-to-pdf.mjs)", path.display()));
    let id = docs::create(core, &format!("{name}.pdf"), pdf.len() as u64)
        .unwrap()
        .doc
        .id;
    let raw = RawChunk {
        doc_id: Some(id.clone()),
        offset: Some("0".into()),
        body: Some(pdf),
    };
    upload::chunk(core, &raw).unwrap();
    docs::confirm(core, &id).unwrap();
    docs::process(core, &id).unwrap();
    id
}

fn usage_of(v: &Value) -> Usage {
    let n = |k: &str| v[k].as_u64().unwrap_or(0);
    Usage {
        input: n("input"),
        output: n("output"),
        cache_read: n("cacheRead"),
        cache_write: n("cacheWrite"),
    }
}

fn today_usage(core: &Composition) -> Usage {
    usage_of(&serde_json::to_value(session::usage(core).unwrap()).unwrap()["today"])
}

fn minus(a: Usage, b: Usage) -> Usage {
    Usage {
        input: a.input.saturating_sub(b.input),
        output: a.output.saturating_sub(b.output),
        cache_read: a.cache_read.saturating_sub(b.cache_read),
        cache_write: a.cache_write.saturating_sub(b.cache_write),
    }
}

/// Asks one question in a fresh chat; returns the channel events.
fn ask_events(core: &Composition, q: &Question, doc: Option<&str>) -> Result<Vec<Value>, Value> {
    let doc_ids: Vec<String> = doc.map(str::to_owned).into_iter().collect();
    let chat = serde_json::to_value(chats::create(core, Some(&doc_ids)).unwrap()).unwrap();
    let chat_id = chat["chat"]["id"].as_str().unwrap().to_owned();
    let mut events = Vec::new();
    let outcome = ask::ask_now(core, &chat_id, (&q.text, Some(MODEL)), &mut |e| {
        events.push(serde_json::to_value(e).unwrap());
        true
    });
    outcome
        .map(|()| events)
        .map_err(|e| serde_json::to_value(e).unwrap())
}

fn of(events: &[Value], name: &str) -> Vec<Value> {
    events
        .iter()
        .filter(|e| e["event"] == name)
        .map(|e| e["data"].clone())
        .collect()
}

fn tool_calls(calls: &[eval_support::recorder::LawCall]) -> Vec<Value> {
    calls
        .iter()
        .map(|c| {
            let provisions: Vec<String> = c
                .provisions
                .iter()
                .map(|(i, l)| format!("{i} \u{b7} {l}"))
                .collect();
            json!({ "tool": c.tool, "query": c.query, "provisions": provisions })
        })
        .collect()
}

fn by_kind(citations: &[Value], kind: &str) -> Vec<Value> {
    let of_kind = citations.iter().filter(|c| c["kind"] == kind);
    of_kind
        .map(|c| json!({ "n": c["n"], "title": c["docTitle"], "page": c["page"] }))
        .collect()
}

fn mentions(q: &Question, text: &str) -> Vec<Value> {
    let lower = text.to_lowercase();
    let found = |m: &String| lower.contains(&m.to_lowercase());
    q.must_mention
        .iter()
        .map(|m| json!({ "phrase": m, "found": found(m) }))
        .collect()
}

/// One report row from a finished (or failed) ask.
fn row(
    q: &Question,
    events: &Result<Vec<Value>, Value>,
    calls: &[eval_support::recorder::LawCall],
    follow_up: Usage,
) -> Value {
    let empty = Vec::new();
    let events = events.as_ref().unwrap_or(&empty);
    let done = of(events, "done").pop().unwrap_or(Value::Null);
    let citations: Vec<Value> = done["message"]["citations"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let text = done["message"]["text"].as_str().unwrap_or("");
    let cites: Vec<Cite> = citations.iter().filter_map(Cite::from_json).collect();
    let (targets, hit) = target_hits(q, calls, &citations);
    let answer = usage_of(&done["usage"]);
    json!({
        "id": q.id, "kind": q.kind, "language": q.language, "question": q.text,
        "toolCalls": tool_calls(calls),
        "searchEvents": of(events, "search"),
        "targets": targets, "hit": hit,
        "citations": { "law": by_kind(&citations, "law"), "doc": by_kind(&citations, "doc") },
        "docPageCited": q.doc_page.map(|p| doc_page_cited(p, &citations)),
        "mustMention": mentions(q, text),
        "uncited": uncited_statutory_sentences(text, &cites),
        "stopReason": done["message"]["stopReason"],
        "answerUsage": done["usage"],
        "followUpUsage": { "input": follow_up.input, "output": follow_up.output },
        "costUsd": cost(&answer, SONNET) + cost(&follow_up, HAIKU),
        "error": of(events, "error").pop().or_else(|| events_error(events)),
        "answer": text,
    })
}

fn events_error(events: &[Value]) -> Option<Value> {
    (!events.iter().any(|e| e["event"] == "done")).then(|| json!("no done event"))
}

fn run_question(core: &Composition, log: &Recorded, q: &Question) -> Value {
    let doc = q.doc.as_deref().map(|name| upload_pdf(core, name));
    log.take();
    let before = today_usage(core);
    let events = ask_events(core, q, doc.as_deref());
    let calls = log.take();
    let spent = minus(today_usage(core), before);
    let answer = events
        .as_ref()
        .ok()
        .and_then(|e| of(e, "done").pop())
        .map_or_else(Usage::default, |d| usage_of(&d["usage"]));
    let mut r = row(q, &events, &calls, minus(spent, answer));
    if let Err(e) = events {
        r["error"] = e;
    }
    if let Some(id) = doc {
        docs::delete(core, &id).unwrap();
    }
    r
}

#[test]
#[ignore = "calls the real Claude API (about US$2-3) and needs desktop/data; run by hand"]
fn eval_every_question_through_the_real_ask_flow() {
    let dir = tempfile::tempdir().unwrap();
    let log = Recorded::default();
    let core = open_app(dir.path(), &log);
    let mut rows = Vec::new();
    let mut spent = 0.0;
    for q in load() {
        let r = run_question(&core, &log, &q);
        spent += r["costUsd"].as_f64().unwrap_or(0.0);
        println!(
            "{} hit={} uncited={} cost=${:.4} total=${spent:.3}",
            q.id,
            r["hit"],
            r["uncited"].as_array().map_or(0, Vec::len),
            r["costUsd"].as_f64().unwrap_or(0.0)
        );
        rows.push(r);
        if spent > BUDGET_USD * 0.95 {
            println!("stopping: spend ${spent:.3} is near the ${BUDGET_USD} ceiling");
            break;
        }
    }
    let day = kristoff_core::util::time::utc_day(kristoff_core::ports::Clock::now_ms(
        &kristoff_adapters::system::SystemClock,
    ));
    let report =
        json!({ "date": day, "model": MODEL, "summary": summary(&rows), "questions": rows });
    let out = desktop_dir().join("target/eval");
    std::fs::create_dir_all(&out).unwrap();
    let path = out.join(format!("report-{day}.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!(
        "report: {}\n{}",
        path.display(),
        serde_json::to_string_pretty(&report["summary"]).unwrap()
    );
}

/// Citations of a saved row; rows from before citation numbers were saved get `n` by order when
/// every citation is a law citation, and `None` otherwise (mixed answers cannot be re-scored).
fn saved_cites(row: &Value) -> Option<Vec<Cite>> {
    let law = row["citations"]["law"].as_array()?;
    let doc = row["citations"]["doc"].as_array()?;
    let numbered = |c: &Value, kind: &str, n: Option<usize>| {
        let n = c["n"]
            .as_u64()
            .or_else(|| n.and_then(|n| u64::try_from(n + 1).ok()))?;
        Cite::from_json(&json!({ "n": n, "kind": kind, "docTitle": c["title"] }))
    };
    let has_n = law.iter().chain(doc).all(|c| c["n"].is_u64());
    if !has_n && !doc.is_empty() {
        return None;
    }
    let laws = law
        .iter()
        .enumerate()
        .map(|(i, c)| numbered(c, "law", Some(i)));
    let docs = doc.iter().map(|c| numbered(c, "doc", None));
    laws.chain(docs).collect()
}

#[test]
#[ignore = "re-scores a saved report with the current checker; set KRISTOFF_EVAL_REPORT"]
fn rescore_a_saved_report() {
    let path = std::env::var("KRISTOFF_EVAL_REPORT").expect("KRISTOFF_EVAL_REPORT");
    let report: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let mut out = Vec::new();
    for row in report["questions"].as_array().unwrap() {
        let text = row["answer"].as_str().unwrap_or("");
        let uncited = saved_cites(row).map(|c| uncited_statutory_sentences(text, &c));
        out.push(json!({ "id": row["id"], "uncited": uncited }));
    }
    let target = format!("{path}.rescored.json");
    std::fs::write(&target, serde_json::to_vec_pretty(&out).unwrap()).unwrap();
    println!("rescored: {target}");
}
