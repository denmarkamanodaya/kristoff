//! `CaseIndex`: read-only search over `phcases.db` (Phase 7, D-20), the adapter behind
//! `kristoff_core::ports::CaseSearch`. FTS5 over passages, int8 kNN over one vector per passage,
//! and exact G.R. hits are merged by reciprocal rank fusion into one hit per decision.

mod lookup;
mod vectors;

use std::path::Path;

use kristoff_core::domain::case::{CaseFilters, CaseHit, CaseView};
use kristoff_core::error::AppResult;
use kristoff_core::ports::CaseSearch;
use rusqlite::params;

use crate::QueryEmbedder;
use crate::case_schema::FTS_RANK;
use crate::docket::gr_key;
use crate::error::LawIndexError;
use crate::fts::two_pass;
use crate::fusion::{RankedList, fuse};
use crate::store::ReadOnlyDb;
use vectors::{LOAD_SQL, PassageVectors};

/// Decisions taken from each of FTS and kNN.
const CANDIDATES: usize = 30;
/// Most hits one search returns (the port's cap).
const MAX_LIMIT: u8 = 8;
/// Weight of an exact G.R. hit in the fusion.
const EXACT_WEIGHT: f64 = 3.0;
/// Passages read from FTS before they are folded into decisions.
const FTS_PASSAGES: i64 = 400;
/// An all-terms FTS pass returning fewer decisions than this falls back to any-term.
const AND_MIN: usize = 10;

/// Read-only handle on `phcases.db`.
pub struct CaseIndex {
    db: ReadOnlyDb<PassageVectors>,
}

fn admits(filters: CaseFilters, year: u16) -> bool {
    filters.year_from.is_none_or(|y| year >= y) && filters.year_to.is_none_or(|y| year <= y)
}

impl CaseIndex {
    /// Opens `phcases_db` read-only and immutable; the passage vectors load on warm or the first
    /// search.
    ///
    /// # Errors
    /// [`LawIndexError::Db`] when `phcases.db` cannot be opened.
    pub fn open(
        phcases_db: &Path,
        embedder: Box<dyn QueryEmbedder>,
    ) -> Result<Self, LawIndexError> {
        ReadOnlyDb::open(phcases_db, embedder).map(|db| Self { db })
    }

    fn vectors(&self) -> Result<&PassageVectors, LawIndexError> {
        self.db.vectors(LOAD_SQL, PassageVectors::push_row)
    }

    /// Best FTS passage per decision: `(case_rowid, passage_id)`, best first.
    fn fts_cases(
        &self,
        query: &str,
        filters: CaseFilters,
    ) -> Result<Vec<(i64, i64)>, LawIndexError> {
        two_pass(query, AND_MIN, |expr| self.fts_pass(expr, filters))
    }

    fn fts_pass(&self, expr: &str, filters: CaseFilters) -> Result<Vec<(i64, i64)>, LawIndexError> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT p.case_rowid, p.id FROM passages_fts JOIN passages p ON p.id = passages_fts.rowid \
             JOIN cases c ON c.id = p.case_rowid WHERE passages_fts MATCH ?1 \
             AND (?2 IS NULL OR c.year >= ?2) AND (?3 IS NULL OR c.year <= ?3) ORDER BY {FTS_RANK} LIMIT ?4"
        ))?;
        let rows = stmt.query_map(
            params![expr, filters.year_from, filters.year_to, FTS_PASSAGES],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )?;
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for row in rows {
            let (case, passage) = row?;
            if seen.insert(case) {
                out.push((case, passage));
                if out.len() == CANDIDATES {
                    break;
                }
            }
        }
        Ok(out)
    }

    /// Hybrid search, one hit per decision, at most 8.
    ///
    /// # Errors
    /// [`LawIndexError`] when the db or the embedder fails.
    pub fn search_cases(
        &self,
        query: &str,
        filters: &CaseFilters,
        limit: u8,
    ) -> Result<Vec<CaseHit>, LawIndexError> {
        let vectors = self.vectors()?;
        let (fts, embedded) = std::thread::scope(|s| {
            let fts = s.spawn(|| self.fts_cases(query, *filters));
            let embedded = self
                .db
                .embedder
                .embed_query(query)
                .map_err(LawIndexError::Embed);
            (
                fts.join()
                    .map_err(|_| LawIndexError::Poisoned)
                    .and_then(|r| r),
                embedded,
            )
        });
        let fts = fts?;
        let knn = vectors.best_cases(&embedded?, |y| admits(*filters, y), CANDIDATES);
        let exact = self.exact_cases(query, *filters)?;
        let take = usize::from(limit.min(MAX_LIMIT));
        fuse_cases(&fts, &knn, &exact)
            .into_iter()
            .take(take)
            .map(|(case, passage)| self.hit(case, passage))
            .collect()
    }

    fn exact_cases(&self, query: &str, filters: CaseFilters) -> Result<Vec<i64>, LawIndexError> {
        let Some(key) = gr_key(query, false) else {
            return Ok(vec![]);
        };
        let conn = self.db.conn()?;
        let mut stmt =
            conn.prepare_cached("SELECT id, year FROM cases WHERE gr_key = ?1 ORDER BY id")?;
        let rows = stmt.query_map([key], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, u16>(1)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (id, year) = row?;
            if admits(filters, year) {
                out.push(id);
            }
        }
        Ok(out)
    }
}

/// Fuses the FTS, kNN and exact lists of `(case_rowid, passage_id)` by RRF; each decision comes
/// with its best passage (FTS first, then kNN), `None` for exact-only hits.
fn fuse_cases(fts: &[(i64, i64)], knn: &[(i64, i64)], exact: &[i64]) -> Vec<(i64, Option<i64>)> {
    let ids = |v: &[(i64, i64)]| v.iter().map(|(c, _)| *c).collect::<Vec<_>>();
    let (fts_ids, knn_ids) = (ids(fts), ids(knn));
    let fused = fuse(&[
        RankedList {
            ids: &fts_ids,
            weight: 1.0,
        },
        RankedList {
            ids: &knn_ids,
            weight: 1.0,
        },
        RankedList {
            ids: exact,
            weight: EXACT_WEIGHT,
        },
    ]);
    let best = |case: i64| {
        fts.iter()
            .chain(knn)
            .find(|(c, _)| *c == case)
            .map(|(_, p)| *p)
    };
    fused
        .into_iter()
        .map(|(case, _)| (case, best(case)))
        .collect()
}

impl CaseSearch for CaseIndex {
    fn warm(&self) -> AppResult<()> {
        self.vectors()?;
        Ok(self.db.embedder.warm().map_err(LawIndexError::Embed)?)
    }

    fn case_count(&self) -> AppResult<u64> {
        let n: i64 = self
            .db
            .conn()?
            .query_row("SELECT count(*) FROM cases", [], |r| r.get(0))
            .map_err(LawIndexError::from)?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    fn search(&self, query: &str, filters: &CaseFilters, limit: u8) -> AppResult<Vec<CaseHit>> {
        Ok(self.search_cases(query, filters, limit)?)
    }

    fn get_case(&self, case: &str, focus: Option<&str>) -> AppResult<Option<CaseView>> {
        Ok(self.case_view(case, focus)?)
    }
}
