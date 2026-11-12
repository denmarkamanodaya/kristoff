//! Reading decisions back: the hit for a decision, and `get_case` with focused passages.

use kristoff_core::domain::case::{CaseHit, CaseView};
use rusqlite::{OptionalExtension, Row, params};

use super::CaseIndex;
use crate::{case_schema::FTS_RANK, docket::gr_key, error::LawIndexError, fts::match_expression};

/// Longest excerpt, in chars.
const EXCERPT_CHARS: usize = 700;
/// Most passages a `get_case` returns (the port's cap).
const MAX_PASSAGES: usize = 3;

const CASE_SELECT: &str =
    "SELECT id, case_id, gr_number, title, date, ponente, source_url FROM cases";

fn excerpt(text: &str) -> String {
    let cut: String = text.chars().take(EXCERPT_CHARS).collect();
    if cut.len() < text.len() {
        format!("{}…", cut.trim_end())
    } else {
        cut
    }
}

fn hit_from_row(row: &Row<'_>) -> rusqlite::Result<(i64, CaseHit)> {
    let url: String = row.get(6)?;
    Ok((
        row.get(0)?,
        CaseHit {
            case_id: row.get(1)?,
            gr_number: row.get(2)?,
            title: row.get(3)?,
            date: row.get(4)?,
            ponente: row.get(5)?,
            excerpt: String::new(),
            source_url: url.starts_with("https://").then_some(url),
        },
    ))
}

impl CaseIndex {
    /// The hit for decision `case`, its excerpt from `passage` (or its opening passage).
    pub(super) fn hit(&self, case: i64, passage: Option<i64>) -> Result<CaseHit, LawIndexError> {
        let conn = self.db.conn()?;
        let (_, mut hit) = conn.query_row(
            &format!("{CASE_SELECT} WHERE id = ?1"),
            [case],
            hit_from_row,
        )?;
        let text: Option<String> = match passage {
            Some(p) => conn
                .query_row("SELECT text FROM passages WHERE id = ?1", [p], |r| r.get(0))
                .optional()?,
            None => None,
        };
        let text = match text {
            Some(t) => t,
            None => conn.query_row(
                "SELECT text FROM passages WHERE case_rowid = ?1 ORDER BY ord LIMIT 1",
                [case],
                |r| r.get(0),
            )?,
        };
        hit.excerpt = excerpt(&text);
        Ok(hit)
    }

    /// Finds a decision by case id, or by G.R. number (`G.R. No. 135981` or `135981`).
    fn resolve(&self, case: &str) -> Result<Option<(i64, CaseHit)>, LawIndexError> {
        let conn = self.db.conn()?;
        let by_id = conn
            .query_row(
                &format!("{CASE_SELECT} WHERE case_id = ?1"),
                [case.trim()],
                hit_from_row,
            )
            .optional()?;
        if by_id.is_some() {
            return Ok(by_id);
        }
        let Some(key) = gr_key(case, true) else {
            return Ok(None);
        };
        // A G.R. number can hold a decision and later resolutions; the main decision is the longest.
        let sql = format!(
            "{CASE_SELECT} WHERE gr_key = ?1 \
             ORDER BY (SELECT count(*) FROM passages p WHERE p.case_rowid = cases.id) DESC, id LIMIT 1"
        );
        Ok(conn.query_row(&sql, [key], hit_from_row).optional()?)
    }

    /// The decision with up to three passages: the ones matching `focus` (in document order),
    /// or its opening passage and its dispositive passage (`WHEREFORE`).
    pub(super) fn case_view(
        &self,
        case: &str,
        focus: Option<&str>,
    ) -> Result<Option<CaseView>, LawIndexError> {
        let Some((rowid, mut hit)) = self.resolve(case)? else {
            return Ok(None);
        };
        let focused = match focus.and_then(match_expression) {
            Some(expr) => self.focused(rowid, &expr)?,
            None => Vec::new(),
        };
        let passages = if focused.is_empty() {
            self.opening(rowid)?
        } else {
            focused
        };
        hit.excerpt = passages.first().map(|p| excerpt(p)).unwrap_or_default();
        Ok(Some(CaseView { hit, passages }))
    }

    fn focused(&self, case: i64, expr: &str) -> Result<Vec<String>, LawIndexError> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT p.ord, p.text FROM passages_fts JOIN passages p ON p.id = passages_fts.rowid \
             WHERE passages_fts MATCH ?1 AND p.case_rowid = ?2 ORDER BY {FTS_RANK} LIMIT ?3"
        ))?;
        let limit = i64::try_from(MAX_PASSAGES).unwrap_or(3);
        let mut rows: Vec<(i64, String)> = stmt
            .query_map(params![expr, case, limit], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        rows.sort_by_key(|(ord, _)| *ord);
        Ok(rows.into_iter().map(|(_, t)| t).collect())
    }

    fn opening(&self, case: i64) -> Result<Vec<String>, LawIndexError> {
        let conn = self.db.conn()?;
        let first: Vec<(i64, String)> = conn
            .prepare_cached(
                "SELECT ord, text FROM passages WHERE case_rowid = ?1 ORDER BY ord LIMIT 2",
            )?
            .query_map([case], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let dispositive: Option<(i64, String)> = conn
            .prepare_cached("SELECT ord, text FROM passages WHERE case_rowid = ?1 AND instr(text, 'WHEREFORE') > 0 ORDER BY ord DESC LIMIT 1")?
            .query_row([case], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?;
        let mut out = first;
        if let Some(d) = dispositive.filter(|d| out.iter().all(|(o, _)| *o != d.0)) {
            out.push(d);
        }
        Ok(out.into_iter().take(MAX_PASSAGES).map(|(_, t)| t).collect())
    }
}
