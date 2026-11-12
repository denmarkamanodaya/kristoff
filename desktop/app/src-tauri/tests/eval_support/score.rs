//! Scoring and cost: target hits, doc page check, token cost and the summary block.

use super::questions::{Question, Target};
use super::recorder::LawCall;
use kristoff_core::model::Usage;
use serde_json::{Value, json};

/// USD per million tokens: input, output, cache write, cache read. Claude API skill price table
/// (cached 2026-11-03); cache writes at 1.25 times input.
pub const SONNET: [f64; 4] = [2.0, 10.0, 2.5, 0.10];
/// The same for Haiku 5.5 (prompts under 100K tokens), used by the follow-up call.
pub const HAIKU: [f64; 4] = [0.10, 0.50, 0.125, 0.01];
/// Results per search the look-up bar counts.
pub const TOP: usize = 8;

fn millions(n: u64) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX)) / 1_000_000.0
}

/// The USD cost of `usage` at `price`.
pub fn cost(usage: &Usage, price: [f64; 4]) -> f64 {
    millions(usage.input) * price[0]
        + millions(usage.output) * price[1]
        + millions(usage.cache_write) * price[2]
        + millions(usage.cache_read) * price[3]
}

fn same(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

fn is(target: &Target, identifier: &str, label: &str) -> bool {
    same(&target.identifier, identifier) && same(&target.label, label)
}

/// Whether `target` was among the first `TOP` provisions of any law call.
pub fn retrieved(target: &Target, calls: &[LawCall]) -> bool {
    calls
        .iter()
        .any(|c| c.provisions.iter().take(TOP).any(|(i, l)| is(target, i, l)))
}

/// Whether a law citation's title (`<identifier> · <label>`) names `target`.
pub fn cited(target: &Target, citations: &[Value]) -> bool {
    citations.iter().filter(|c| c["kind"] == "law").any(|c| {
        let title = c["docTitle"].as_str().unwrap_or("");
        title
            .split_once(" \u{b7} ")
            .is_some_and(|(i, l)| is(target, i, l))
    })
}

/// Per-target `{target, retrieved, cited}` and whether the question's bar holds: every target
/// retrieved for a look-up, any target retrieved or cited for a concept or mixed question.
pub fn target_hits(q: &Question, calls: &[LawCall], citations: &[Value]) -> (Vec<Value>, bool) {
    let rows: Vec<(bool, bool, Value)> = q
        .targets
        .iter()
        .map(|t| {
            let (r, c) = (retrieved(t, calls), cited(t, citations));
            (r, c, json!({ "target": format!("{} \u{b7} {}", t.identifier, t.label), "retrieved": r, "cited": c }))
        })
        .collect();
    let hit = if q.kind == "lookup" {
        rows.iter().all(|(r, _, _)| *r)
    } else {
        rows.iter().any(|(r, c, _)| *r || *c)
    };
    (rows.into_iter().map(|(_, _, v)| v).collect(), hit)
}

/// Whether a doc citation points at the expected page.
pub fn doc_page_cited(page: u32, citations: &[Value]) -> bool {
    citations.iter().any(|c| {
        c["kind"] == "doc"
            && c["page"].as_u64().is_some_and(|p| u64::from(page) >= p)
            && c["endPage"]
                .as_u64()
                .or(c["page"].as_u64())
                .is_some_and(|e| u64::from(page) <= e)
    })
}

fn rate(hits: usize, total: usize) -> Value {
    if total == 0 {
        return Value::Null;
    }
    json!({ "hits": hits, "total": total, "rate": f64::from(u32::try_from(hits).unwrap_or(0)) / f64::from(u32::try_from(total).unwrap_or(1)) })
}

/// The summary over per-question rows (each with `kind`, `language`, `hit`, `uncited`, `costUsd`,
/// and `docPageCited` for mixed questions).
pub fn summary(rows: &[Value]) -> Value {
    let pick = |f: &dyn Fn(&Value) -> bool| rows.iter().filter(|r| f(r)).collect::<Vec<_>>();
    let hits = |rs: &[&Value]| rate(rs.iter().filter(|r| r["hit"] == true).count(), rs.len());
    let lookups = pick(&|r| r["kind"] == "lookup");
    let statute = pick(&|r| r["kind"] != "mixed");
    let mixed = pick(&|r| r["kind"] == "mixed");
    let total_cost: f64 = rows.iter().filter_map(|r| r["costUsd"].as_f64()).sum();
    let statute_cost: f64 = statute.iter().filter_map(|r| r["costUsd"].as_f64()).sum();
    let avg = statute_cost / f64::from(u32::try_from(statute.len().max(1)).unwrap_or(1));
    let uncited: usize = rows
        .iter()
        .map(|r| r["uncited"].as_array().map_or(0, Vec::len))
        .sum();
    let by_lang = |lang: &str| {
        let rs = pick(&|r| r["language"] == lang);
        let n: usize = rs
            .iter()
            .map(|r| r["uncited"].as_array().map_or(0, Vec::len))
            .sum();
        json!({ "targets": hits(&rs), "uncitedStatutoryClaims": n, "ids": rs.iter().map(|r| r["id"].clone()).collect::<Vec<_>>() })
    };
    let lookup_rate = hits(&lookups)["rate"].as_f64().unwrap_or(0.0);
    json!({
        "lookupsRetrievedTop8": hits(&lookups),
        "conceptTargetsHit": hits(&pick(&|r| r["kind"] == "concept")),
        "mixedTargetsHit": hits(&mixed),
        "mixedDocPageCited": rate(mixed.iter().filter(|r| r["docPageCited"] == true).count(), mixed.len()),
        "uncitedStatutoryClaims": uncited,
        "avgCostPerStatuteQuestionUsd": avg,
        "totalCostUsd": total_cost,
        "errors": rows.iter().filter(|r| !r["error"].is_null()).count(),
        "byLanguage": { "en": by_lang("en"), "taglish": by_lang("taglish"), "tl": by_lang("tl") },
        "bars": {
            "lookupsAtLeast90Percent": lookup_rate >= 0.9,
            "zeroUncitedClaims": uncited == 0,
            "avgStatuteCostAtMost5Cents": avg <= 0.05,
        },
    })
}
