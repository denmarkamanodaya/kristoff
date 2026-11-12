//! Live statute-tool check (phase-4 exit), IGNORED by default: it calls the real Claude API (Sonnet
//! 5.5) with the key baked from `desktop/.env.local` and searches the real law index
//! (`KRISTOFF_PHLAWS_DB`, `KRISTOFF_E5_MODEL_DIR`). Run by hand:
//! `cargo test -p kristoff-app --test live_statutes -- --ignored`, with `KRISTOFF_LIVE_REPORT`
//! naming the JSON report file. Two questions: statutes only, then with a decision PDF attached.

// Test helpers outside #[test] fns: a failed setup must fail the test loudly.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use kristoff_app::commands::upload::RawChunk;
use kristoff_app::commands::{ask, chats, docs, upload};
use kristoff_app::composition::Composition;
use kristoff_app::law::{LawPaths, open_law_index};
use serde_json::{Value, json};

const QUESTION: &str = "Under RA 9262, is battered woman syndrome a defense? Cite the section.";

fn page_text(p: u32) -> String {
    match p {
        2 => "Page 2. The accused-appellant invoked battered woman syndrome under Section 26 of Republic Act No. 9262 as a complete defense.".into(),
        3 => "Page 3. The Court holds that the defense was proven: the battering cycle occurred at least twice, so the accused incurs no criminal liability.".into(),
        _ => format!("Page {p}. People v. Testcase, a decision of the Supreme Court."),
    }
}

fn open_app(dir: &std::path::Path) -> Composition {
    let core = Composition::open(dir, kristoff_app::ANTHROPIC_KEY).unwrap();
    let paths = LawPaths::resolve(dir, |k| std::env::var_os(k));
    core.install_law(open_law_index(&paths).unwrap()).unwrap();
    core
}

fn uploaded_decision(core: &Composition) -> String {
    let pdf = kristoff_testkit::pdf::make_pdf_with_text(4, |_| 0, page_text);
    let id = docs::create(core, "People v. Testcase.pdf", pdf.len() as u64)
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

fn ask_sonnet(core: &Composition, doc_ids: &[String]) -> Vec<Value> {
    let chat = serde_json::to_value(chats::create(core, Some(doc_ids)).unwrap()).unwrap();
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
        "eventNames": events.iter().filter_map(|e| e["event"].as_str()).collect::<Vec<_>>(),
        "searches": of("search"),
        "citations": of("citation"),
        "error": of("error"),
        "usage": done["usage"],
        "stopReason": done["message"]["stopReason"],
        "text": done["message"]["text"],
    })
}

fn kinds(report: &Value) -> Vec<String> {
    report["citations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["kind"].as_str().unwrap_or("").to_owned())
        .collect()
}

#[test]
#[ignore = "calls the real Claude API (costs money) and needs desktop/data; run by hand"]
fn statute_search_cites_ra_9262_sec_26_alone_and_alongside_a_pdf() {
    let dir = tempfile::tempdir().unwrap();
    let core = open_app(dir.path());

    let law_only = summary(&ask_sonnet(&core, &[]));
    let doc = uploaded_decision(&core);
    let both = summary(&ask_sonnet(&core, std::slice::from_ref(&doc)));
    docs::delete(&core, &doc).unwrap();

    let report = json!({ "lawOnly": law_only, "withPdf": both });
    if let Ok(path) = std::env::var("KRISTOFF_LIVE_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    assert!(
        !law_only["searches"].as_array().unwrap().is_empty(),
        "{report}"
    );
    let sec26 = law_only["citations"].as_array().unwrap().iter().any(|c| {
        let t = c["docTitle"].as_str().unwrap_or("");
        c["kind"] == "law" && t.contains("9262") && t.contains("26")
    });
    assert!(sec26, "{report}");
    let k = kinds(&both);
    assert!(
        k.iter().any(|x| x == "doc") && k.iter().any(|x| x == "law"),
        "{report}"
    );
}
