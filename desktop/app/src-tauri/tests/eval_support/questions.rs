//! `contracts/fixtures/eval/questions.json`, read into plain structs.

use serde_json::Value;
use std::path::{Path, PathBuf};

/// One expected provision: identifier and label as stored in `phlaws.db`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub identifier: String,
    pub label: String,
}

/// One eval question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub id: String,
    /// `lookup`, `concept` or `mixed`.
    pub kind: String,
    /// `en`, `tl` or `taglish`.
    pub language: String,
    /// The question text.
    pub text: String,
    pub targets: Vec<Target>,
    /// For mixed questions: the doc's base name (`m01-vawc-psychological`).
    pub doc: Option<String>,
    /// For mixed questions: the page the answer should cite.
    pub doc_page: Option<u32>,
    /// For mixed questions: phrases the answer should mention.
    pub must_mention: Vec<String>,
}

fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().to_owned()
}

fn question(v: &Value) -> Question {
    let targets = v["targets"].as_array().map_or_else(Vec::new, |ts| {
        ts.iter()
            .map(|t| Target {
                identifier: text(t, "identifier"),
                label: text(t, "label"),
            })
            .collect()
    });
    let doc = v["doc"]["file"].as_str().map(|f| {
        let name = Path::new(f).file_stem().unwrap_or_default();
        name.to_string_lossy().into_owned()
    });
    let must_mention = v["docExpect"]["mustMention"]
        .as_array()
        .map_or_else(Vec::new, |m| {
            m.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        });
    Question {
        id: text(v, "id"),
        kind: text(v, "kind"),
        language: text(v, "language"),
        text: text(v, "question"),
        targets,
        doc,
        doc_page: v["docExpect"]["page"]
            .as_u64()
            .and_then(|p| u32::try_from(p).ok()),
        must_mention,
    }
}

/// The repo's `contracts/fixtures/eval/` folder.
pub fn eval_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../contracts/fixtures/eval")
}

/// Every question of the committed set, in file order.
///
/// # Panics
/// When the file is missing or not JSON.
pub fn load() -> Vec<Question> {
    let raw = std::fs::read(eval_dir().join("questions.json")).expect("questions.json");
    let json: Value = serde_json::from_slice(&raw).expect("questions.json is JSON");
    json["questions"]
        .as_array()
        .expect("questions array")
        .iter()
        .map(question)
        .collect()
}
