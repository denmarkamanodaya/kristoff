//! Candidate lists: BM25 over `chunks_fts`, and exact hits for cited laws and provisions.

use kristoff_core::domain::law::LawFilters;
use kristoff_core::domain::law::reference::{LawRef, References};
use rusqlite::params;

use super::vectors::{Scored, Vectors};
use super::{CANDIDATES, LawIndex};
use crate::chunk_kind::{is_heading_only, opens_provision};
use crate::error::LawIndexError;
use crate::fts::match_expression;
use crate::schema::FTS_RANK;

/// Exact citation hits kept per query.
const EXACT_LIMIT: usize = 5;
/// Candidates pooled from inside a cited law before openers are moved to the front.
const LAW_POOL: usize = 20;

/// One query's text, parsed citations, filters and the loaded vectors.
#[derive(Clone, Copy)]
pub(super) struct QueryCtx<'a> {
    pub(super) text: &'a str,
    pub(super) refs: &'a References,
    pub(super) filters: &'a LawFilters,
    pub(super) vectors: &'a Vectors,
}

impl LawIndex {
    /// BM25 candidates ranked by [`FTS_RANK`], optionally limited to one law; heading-only
    /// chunks are skipped.
    pub(super) fn fts_candidates(
        &self,
        query: &str,
        filters: &LawFilters,
        law: Option<i64>,
    ) -> Result<Vec<i64>, LawIndexError> {
        let Some(expr) = match_expression(query) else {
            return Ok(vec![]);
        };
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT c.id, l.number, l.year, c.sec_num IS NOT NULL, substr(c.text, 1, 200) \
             FROM chunks_fts JOIN chunks c ON c.id = chunks_fts.rowid \
             JOIN laws l ON l.id = c.law_id \
             WHERE chunks_fts MATCH ?1 AND (?2 IS NULL OR c.law_id = ?2) \
             ORDER BY {FTS_RANK} LIMIT 200"
        ))?;
        let rows = stmt.query_map(params![expr, law], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get::<_, bool>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, number, year, has_section, head) = row?;
            if filters.admits(number, year) && !is_heading_only(has_section, &head) {
                out.push(id);
                if out.len() == CANDIDATES {
                    break;
                }
            }
        }
        Ok(out)
    }

    pub(super) fn law_ids_for(&self, law: LawRef) -> Result<Vec<i64>, LawIndexError> {
        let conn = self.conn()?;
        let (sql, arg) = match law {
            LawRef::RepublicAct(n) => ("SELECT id FROM laws WHERE number = ?1", i64::from(n)),
            LawRef::Constitution => ("SELECT id FROM laws WHERE is_constitution = ?1", 1),
        };
        let mut stmt = conn.prepare_cached(sql)?;
        let ids = stmt
            .query_map([arg], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(ids)
    }

    /// Chunks matching a cited law (and its cited section/article), best first.
    pub(super) fn exact_hits(
        &self,
        ctx: &QueryCtx<'_>,
        scored: &[Scored],
    ) -> Result<Vec<i64>, LawIndexError> {
        let mut law_ids = Vec::new();
        for law in &ctx.refs.laws {
            law_ids.extend(self.law_ids_for(*law)?);
        }
        if law_ids.is_empty() {
            return Ok(vec![]);
        }
        if ctx.refs.section().is_some() || ctx.refs.article().is_some() {
            return self.provision_hits(&law_ids, ctx.refs);
        }
        // Law cited without a provision: best FTS and kNN hits inside that law, interleaved,
        // provision openers first.
        let mut fts = Vec::new();
        for law in &law_ids {
            fts.extend(self.fts_candidates(ctx.text, ctx.filters, Some(*law))?);
        }
        let knn = scored
            .iter()
            .filter(|(_, law, _)| law_ids.contains(law))
            .map(|(id, _, _)| *id);
        let mut pool: Vec<i64> = Vec::new();
        for id in interleave(fts.into_iter(), knn) {
            if !pool.contains(&id) {
                pool.push(id);
            }
            if pool.len() == LAW_POOL {
                break;
            }
        }
        let (mut out, fragments): (Vec<i64>, Vec<i64>) =
            pool.into_iter().partition(|id| ctx.vectors.is_opener(*id));
        out.extend(fragments);
        out.truncate(EXACT_LIMIT);
        Ok(out)
    }

    /// Chunks of `law_ids` carrying the cited section/article. Chunks that open the provision
    /// (`SEC. 5. ...`) win; cross-reference fragments (`Section 5 hereof ...`) are kept only when
    /// no opener exists.
    pub(super) fn provision_hits(
        &self,
        law_ids: &[i64],
        refs: &References,
    ) -> Result<Vec<i64>, LawIndexError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT id, substr(text, 1, 80) FROM chunks WHERE law_id = ?1 AND part = 0 \
             AND (?2 IS NULL OR sec_num = ?2) AND (?3 IS NULL OR art_num = ?3) ORDER BY ord LIMIT 30",
        )?;
        let number = refs.section().or(refs.article());
        let (mut openers, mut others) = (Vec::new(), Vec::new());
        for law in law_ids {
            let rows = stmt.query_map(params![law, refs.section(), refs.article()], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (id, head) = row?;
                if number.is_some_and(|n| opens_provision(&head, n)) {
                    openers.push(id);
                } else {
                    others.push(id);
                }
            }
        }
        let mut out = if openers.is_empty() { others } else { openers };
        out.truncate(EXACT_LIMIT);
        Ok(out)
    }
}

/// Alternates items from two iterators until both are exhausted.
fn interleave<T>(
    a: impl Iterator<Item = T>,
    b: impl Iterator<Item = T>,
) -> impl Iterator<Item = T> {
    let (mut a, mut b) = (a.fuse(), b.fuse());
    let mut take_a = true;
    std::iter::from_fn(move || {
        let next = if take_a {
            a.next().or_else(|| b.next())
        } else {
            b.next().or_else(|| a.next())
        };
        take_a = !take_a;
        next
    })
}
