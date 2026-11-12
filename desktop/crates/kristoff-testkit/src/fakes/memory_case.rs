//! `MemoryCaseSearch`: an in-memory `CaseSearch` over a few fictional decisions. Ranking is crude
//! (shared words); the contract suite pins only what the real index promises too.

use kristoff_core::domain::case::{CaseFilters, CaseHit, CaseView};
use kristoff_core::error::AppResult;
use kristoff_core::ports::CaseSearch;

/// One stored decision: its hit fields and its passages in document order.
#[derive(Debug, Clone)]
pub struct FakeDecision {
    /// The decision as a hit (its `excerpt` is the first passage).
    pub hit: CaseHit,
    /// Its passages.
    pub passages: Vec<String>,
}

/// In-memory case index.
#[derive(Debug, Clone, Default)]
pub struct MemoryCaseSearch {
    decisions: Vec<FakeDecision>,
}

/// `(case id, G.R. number, title, date)` of one fictional decision.
type Meta<'a> = (&'a str, &'a str, &'a str, &'a str);

fn decision((id, gr, title, date): Meta<'_>, passages: &[&str]) -> FakeDecision {
    FakeDecision {
        hit: CaseHit {
            case_id: id.to_owned(),
            gr_number: Some(gr.to_owned()),
            title: title.to_owned(),
            date: date.to_owned(),
            ponente: Some("Fictional, J.".to_owned()),
            excerpt: passages[0].to_owned(),
            source_url: Some(format!("https://elibrary.judiciary.gov.ph/fake/{id}")),
        },
        passages: passages.iter().map(|p| (*p).to_owned()).collect(),
    }
}

/// Only digits, so `G.R. No. 135981` and `135981` compare equal.
fn digits(text: &str) -> String {
    text.chars().filter(char::is_ascii_digit).collect()
}

fn year(date: &str) -> Option<u16> {
    date.get(..4)?.parse().ok()
}

impl MemoryCaseSearch {
    /// Three fictional decisions; [`crate::contract::SAMPLE_CASE`] names the first.
    #[must_use]
    pub fn sample() -> Self {
        let bws = [
            "The accused-appellant invoked battered woman syndrome as a defense.",
            "The cycle of violence has three phases: tension building, acute battering and reconciliation.",
            "The Court holds that the syndrome was proven and the accused incurs no criminal liability.",
            "WHEREFORE, the accused-appellant is ACQUITTED.",
        ];
        let libel = [
            "The petitioner posted defamatory statements online.",
            "Cyber libel requires publication through a computer system.",
        ];
        let tort = [
            "The bus driver ran a red light.",
            "The employer did not prove the diligence of a good father of a family.",
        ];
        Self {
            decisions: vec![
                decision(
                    (
                        "d-1001",
                        "G.R. No. 900001",
                        "People v. Testcase",
                        "2004-01-15",
                    ),
                    &bws,
                ),
                decision(
                    ("d-1002", "G.R. No. 900002", "Cruz v. Santos", "2015-06-30"),
                    &libel,
                ),
                decision(
                    (
                        "d-1003",
                        "G.R. No. 900003",
                        "Reyes v. Transit Co.",
                        "1999-03-02",
                    ),
                    &tort,
                ),
            ],
        }
    }

    fn score(&self, i: usize, query: &str) -> usize {
        let d = &self.decisions[i];
        let text = format!("{} {}", d.hit.title, d.passages.join(" ")).to_lowercase();
        query
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 2 && text.contains(w))
            .count()
    }

    fn find(&self, case: &str) -> Option<&FakeDecision> {
        let wanted = digits(case);
        self.decisions.iter().find(|d| {
            d.hit.case_id == case
                || (!wanted.is_empty()
                    && d.hit
                        .gr_number
                        .as_deref()
                        .is_some_and(|g| digits(g) == wanted))
        })
    }
}

impl CaseSearch for MemoryCaseSearch {
    fn warm(&self) -> AppResult<()> {
        Ok(())
    }

    fn case_count(&self) -> AppResult<u64> {
        Ok(self.decisions.len() as u64)
    }

    fn search(&self, query: &str, filters: &CaseFilters, limit: u8) -> AppResult<Vec<CaseHit>> {
        let within = |d: &FakeDecision| {
            let y = year(&d.hit.date);
            filters.year_from.is_none_or(|f| y.is_some_and(|y| y >= f))
                && filters.year_to.is_none_or(|t| y.is_some_and(|y| y <= t))
        };
        let mut scored: Vec<(usize, usize)> = (0..self.decisions.len())
            .map(|i| (i, self.score(i, query)))
            .filter(|(i, s)| *s > 0 && within(&self.decisions[*i]))
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        Ok(scored
            .into_iter()
            .take(usize::from(limit.min(8)))
            .map(|(i, _)| self.decisions[i].hit.clone())
            .collect())
    }

    fn get_case(&self, case: &str, focus: Option<&str>) -> AppResult<Option<CaseView>> {
        let Some(d) = self.find(case) else {
            return Ok(None);
        };
        let words: Vec<String> = focus
            .unwrap_or("")
            .to_lowercase()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        let matching: Vec<String> = d
            .passages
            .iter()
            .filter(|p| words.iter().any(|w| p.to_lowercase().contains(w)))
            .take(3)
            .cloned()
            .collect();
        let passages = if matching.is_empty() {
            d.passages.iter().take(3).cloned().collect()
        } else {
            matching
        };
        Ok(Some(CaseView {
            hit: d.hit.clone(),
            passages,
        }))
    }
}
