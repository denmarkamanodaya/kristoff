//! Live end-to-end check (phase-2 exit), IGNORED by default: it calls the real Claude API with the
//! key from `KRISTOFF_LIVE_KEY` (read at run time) and costs a few cents. Run by hand:
//! `cargo test -p kristoff-app --test live -- --ignored --nocapture`, with `KRISTOFF_LIVE_REPORT`
//! naming the JSON report file. Flow: a generated 150-page PDF over 30 MiB is uploaded in 8 MiB
//! chunks, confirmed, split into two parts and uploaded to the Files API; part 2 is attached and
//! asked about with Haiku 5.5; the page citation must map back to page 137; the doc is deleted.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use kristoff_app::commands::upload::RawChunk;
use kristoff_app::commands::{ask, chats, docs, upload};
use kristoff_app::composition::Composition;
use serde_json::{Value, json};

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

const HOLDING_PAGE: u32 = 137;
const HOLDING: &str = "The Court holds that the accused acted in self-defense because the battering cycle was proven.";

fn page_text(p: u32) -> String {
    if p == HOLDING_PAGE {
        format!("Page {p}. {HOLDING}")
    } else {
        format!("Page {p}. Routine procedural matters of People v. Testcase.")
    }
}

fn upload_all(core: &Composition, pdf: &[u8]) -> String {
    let id = docs::create(core, "People v. Testcase.pdf", pdf.len() as u64)
        .unwrap()
        .doc
        .id;
    for (i, chunk) in pdf.chunks(8 * 1024 * 1024).enumerate() {
        let raw = RawChunk {
            doc_id: Some(id.clone()),
            offset: Some((i * 8 * 1024 * 1024).to_string()),
            body: Some(chunk.to_vec()),
        };
        upload::chunk(core, &raw).unwrap();
    }
    id
}

fn ask_live(core: &Composition, chat_id: &str) -> Vec<Value> {
    let mut events = Vec::new();
    let question = "On which page does the Court state its holding on self-defense? Quote it.";
    ask::ask_now(
        core,
        chat_id,
        (question, Some("claude-haiku-5-5")),
        &mut |e| {
            events.push(serde_json::to_value(e).unwrap());
            true
        },
    )
    .unwrap();
    events
}

/// What the run produced, as the JSON report.
fn report(pdf_bytes: usize, ready: &Value, events: &[Value], deleted: bool) -> Value {
    let find = |name: &str| {
        events
            .iter()
            .find(|e| e["event"] == name)
            .map(|e| e["data"].clone())
    };
    let names: Vec<&str> = events.iter().filter_map(|e| e["event"].as_str()).collect();
    let citations: Vec<&Value> = events
        .iter()
        .filter(|e| e["event"] == "citation")
        .map(|e| &e["data"])
        .collect();
    let done = find("done");
    json!({
        "pdfBytes": pdf_bytes, "doc": ready["doc"], "eventNames": names, "citations": citations,
        "followUps": find("followups").map(|f| f["followUps"].clone()),
        "doneUsage": done.as_ref().map(|d| d["usage"].clone()),
        "answerText": done.as_ref().map(|d| d["message"]["text"].clone()),
        "deleted": deleted,
    })
}

#[test]
#[ignore = "calls the real Claude API (costs money); run by hand"]
fn a_large_pdf_splits_uploads_and_answers_with_a_mapped_page_citation() {
    let dir = tempfile::tempdir().unwrap();
    let core = live_composition(dir.path());
    let pdf = kristoff_testkit::pdf::make_pdf_with_text(150, |_| 230_000, page_text);

    let id = upload_all(&core, &pdf);
    let confirmed = serde_json::to_value(docs::confirm(&core, &id).unwrap()).unwrap();
    let ready = serde_json::to_value(docs::process(&core, &id).unwrap()).unwrap();
    let chat = chats::create(&core, Some(&[format!("{id}:p2")]))
        .unwrap()
        .chat
        .summary
        .id;
    let events = ask_live(&core, &chat);
    let deleted = docs::delete(&core, &id).is_ok();

    let report = report(pdf.len(), &ready, &events, deleted);
    if let Ok(path) = std::env::var("KRISTOFF_LIVE_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    assert!(pdf.len() > 31_457_280);
    assert_eq!(confirmed["doc"]["status"], "processing");
    assert_eq!(ready["doc"]["parts"].as_array().unwrap().len(), 2);
    assert_eq!(
        report["eventNames"].as_array().unwrap().last(),
        Some(&json!("done"))
    );
    let cited = report["citations"].as_array().unwrap();
    assert!(cited.iter().any(|c| c["page"] == HOLDING_PAGE), "{report}");
    assert!(deleted);
}
