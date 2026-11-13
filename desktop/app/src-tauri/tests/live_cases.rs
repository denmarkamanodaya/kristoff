//! Live statute + case check (phase-7 exit), IGNORED by default: one Sonnet 5.5 question that needs
//! both a code provision and a Supreme Court decision, through the real composition over the real
//! `phlaws.db` and `phcases.db`. The key is read at run time from `KRISTOFF_LIVE_KEY` (never baked
//! into the build). Run by hand:
//! `KRISTOFF_LIVE_KEY=... KRISTOFF_LIVE_REPORT=report.json cargo test -p kristoff-app --test
//! live_cases -- --ignored`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use kristoff_app::commands::{ask, chats};
use kristoff_app::composition::Composition;
use kristoff_app::law::{LawPaths, open_and_install_all};
use serde_json::{Value, json};

const QUESTION: &str = "Under the Revised Penal Code, what did Article 335 on rape originally \
                        say, and which Supreme Court decision addressed the sweetheart defense in \
                        a rape case? Cite the article and the G.R. number.";

/// USD per million tokens for Sonnet 5.5: input, output, cache write, cache read.
const SONNET: [f64; 4] = [2.0, 10.0, 2.5, 0.10];

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn cost(usage: &Value) -> f64 {
    let n = |k: &str| usage[k].as_f64().unwrap_or(0.0) / 1_000_000.0;
    n("input") * SONNET[0]
        + n("output") * SONNET[1]
        + n("cacheWrite") * SONNET[2]
        + n("cacheRead") * SONNET[3]
}

/// The real composition over both real indexes, warmed, with the key given at run time.
fn open_app(dir: &std::path::Path, key: &str) -> Composition {
    let core = Composition::open(dir, key).unwrap();
    let paths = LawPaths {
        db: data().join("phlaws.db"),
        cases_db: data().join("phcases.db"),
        model_dir: data().join("model"),
    };
    open_and_install_all(&core, &paths).unwrap();
    core.warm_law().unwrap();
    core
}

fn ask_sonnet(core: &Composition) -> Vec<Value> {
    let chat = serde_json::to_value(chats::create(core, None).unwrap()).unwrap();
    let chat_id = chat["chat"]["id"].as_str().unwrap().to_owned();
    let mut events = Vec::new();
    ask::ask_now(
        core,
        &chat_id,
        (QUESTION, Some("claude-sonnet-5-5")),
        &mut |e| {
            events.push(serde_json::to_value(e).unwrap());
            true
        },
    )
    .unwrap();
    events
}

fn summary(events: &[Value]) -> Value {
    let of = |name: &str| -> Vec<Value> {
        events
            .iter()
            .filter(|e| e["event"] == name)
            .map(|e| e["data"].clone())
            .collect()
    };
    let done = of("done").pop().unwrap_or(Value::Null);
    json!({
        "searches": of("search"),
        "citations": of("citation"),
        "error": of("error"),
        "usage": done["usage"],
        "costUsd": cost(&done["usage"]),
        "stopReason": done["message"]["stopReason"],
        "text": done["message"]["text"],
    })
}

#[test]
#[ignore = "calls the real Claude API (costs money) and needs desktop/data; run by hand"]
fn one_question_cites_a_code_provision_and_a_decision() {
    let key = std::env::var("KRISTOFF_LIVE_KEY").expect("KRISTOFF_LIVE_KEY");
    let dir = tempfile::tempdir().unwrap();
    let core = open_app(dir.path(), &key);

    let report = summary(&ask_sonnet(&core));

    if let Ok(path) = std::env::var("KRISTOFF_LIVE_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    let kinds: Vec<_> = report["citations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["kind"].clone())
        .collect();
    assert!(
        kinds.contains(&json!("law")) && kinds.contains(&json!("case")),
        "{report}"
    );
}
