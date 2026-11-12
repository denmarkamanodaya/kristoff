//! `LawIndex`: read-only hybrid search over `phlaws.db` (docs/desktop/phases/phase-3-law-index.md
//! §2), the adapter behind `kristoff_core::ports::LawSearch`.
//!
//! FTS5 BM25 and int8 kNN run in parallel (30 candidates each); citations found by the core
//! reference parser add an exact-hit list; reciprocal rank fusion (K = 60) merges them, and only
//! the best part of each provision is kept. No row type leaves this module.

mod article;
mod candidates;
mod vectors;

use std::path::Path;

use kristoff_core::domain::law::reference::parse_references;
use kristoff_core::domain::law::{ArticleView, LawFilters, LawHit, LawRef, LawSummary};
use kristoff_core::error::AppResult;
use kristoff_core::ports::LawSearch;
use rusqlite::{OptionalExtension, Row};

use crate::error::LawIndexError;
use crate::fusion::{RankedList, fuse};
use crate::store::ReadOnlyDb;
use candidates::QueryCtx;
use vectors::{LOAD_SQL, Vectors};

/// Candidates taken from each of FTS and kNN.
const CANDIDATES: usize = 30;
/// Weight of the exact-hit list in the fusion; above FTS + kNN together so a cited section wins.
const EXACT_WEIGHT: f64 = 3.0;

/// Embeds a search query (the `"query: "` prefix is the embedder's job).
pub trait QueryEmbedder: Send + Sync {
    /// Returns a unit-length vector of 384 floats.
    ///
    /// # Errors
    /// A message describing why embedding failed.
    fn embed_query(&self, text: &str) -> Result<Vec<f32>, String>;

    /// Runs one throw-away query so GPU kernels compile before the first real search.
    ///
    /// # Errors
    /// A message describing why embedding failed.
    fn warm(&self) -> Result<(), String> {
        self.embed_query("warm-up").map(|_| ())
    }
}

/// Read-only handle on `phlaws.db`.
pub struct LawIndex {
    db: ReadOnlyDb<Vectors>,
}

/// Columns of a hit, read by [`hit_from_row`]; `overlap` is column 14, the source name 15 and the
/// edition 16.
const HIT_SELECT: &str = "SELECT c.id, l.identifier, l.title, l.number, l.year, c.label, c.text, \
     l.source_url, l.status, c.truncated, c.law_id, c.ord, c.prov, c.part, c.overlap, s.name, \
     l.edition FROM chunks c JOIN laws l ON l.id = c.law_id JOIN sources s ON s.id = l.dataset_id";

fn hit_from_row(row: &Row<'_>) -> rusqlite::Result<LawHit> {
    Ok(LawHit {
        chunk_id: row.get(0)?,
        law: LawSummary {
            identifier: row.get(1)?,
            title: row.get(2)?,
            number: row.get(3)?,
            year: row.get(4)?,
        },
        label: row.get(5)?,
        text: row.get(6)?,
        source_url: row.get(7)?,
        status: row.get(8)?,
        source: row.get(15)?,
        edition: row.get(16)?,
        truncated: row.get(9)?,
        part: row.get(13)?,
        score: 0.0,
    })
}

impl LawIndex {
    /// Opens `phlaws_db` read-only and immutable. Vectors load on [`LawIndex::warm`] or the first
    /// search.
    ///
    /// # Errors
    /// [`LawIndexError::Db`] when `phlaws.db` cannot be opened.
    pub fn open(phlaws_db: &Path, embedder: Box<dyn QueryEmbedder>) -> Result<Self, LawIndexError> {
        ReadOnlyDb::open(phlaws_db, embedder).map(|db| Self { db })
    }

    /// Loads every vector and warms the query embedder, so the first search is fast.
    ///
    /// # Errors
    /// [`LawIndexError`] when reading fails, a vector is malformed or the embedder fails.
    pub fn warm_index(&self) -> Result<(), LawIndexError> {
        self.vectors()?;
        self.db.embedder.warm().map_err(LawIndexError::Embed)
    }

    fn vectors(&self) -> Result<&Vectors, LawIndexError> {
        self.db.vectors(LOAD_SQL, Vectors::push_row)
    }

    /// Hybrid search: FTS5 + kNN + exact citation hits, fused by RRF, one hit per provision.
    ///
    /// # Errors
    /// [`LawIndexError`] when the db or the embedder fails.
    pub fn search_index(
        &self,
        query: &str,
        filters: &LawFilters,
        limit: usize,
    ) -> Result<Vec<LawHit>, LawIndexError> {
        let vectors = self.vectors()?;
        let refs = parse_references(query);
        let (fts, query_vector) = self.fts_and_embedding(query, filters)?;
        let scored = vectors.scores(&query_vector, filters);
        let knn_top: Vec<i64> = scored
            .iter()
            .take(CANDIDATES)
            .map(|(id, _, _)| *id)
            .collect();
        let ctx = QueryCtx {
            text: query,
            refs: &refs,
            filters,
            vectors,
        };
        let exact = self.exact_hits(&ctx, &scored)?;
        let fused = fuse(&[
            RankedList {
                ids: &fts,
                weight: 1.0,
            },
            RankedList {
                ids: &knn_top,
                weight: 1.0,
            },
            RankedList {
                ids: &exact,
                weight: EXACT_WEIGHT,
            },
        ]);
        self.load_hits(&vectors.one_per_provision(fused), limit)
    }

    /// Runs the BM25 query and the query embedding in parallel.
    fn fts_and_embedding(
        &self,
        query: &str,
        filters: &LawFilters,
    ) -> Result<(Vec<i64>, Vec<f32>), LawIndexError> {
        std::thread::scope(|s| {
            let fts = s.spawn(|| self.fts_candidates(query, filters, None));
            let embedded = self
                .db
                .embedder
                .embed_query(query)
                .map_err(LawIndexError::Embed);
            let fts = fts.join().map_err(|_| LawIndexError::Poisoned)?;
            Ok((fts?, embedded?))
        })
    }

    /// Reads the first `limit` fused ids as hits, carrying their scores.
    fn load_hits(&self, fused: &[(i64, f64)], limit: usize) -> Result<Vec<LawHit>, LawIndexError> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare_cached(&format!("{HIT_SELECT} WHERE c.id = ?1"))?;
        let mut hits = Vec::with_capacity(limit);
        for &(id, score) in fused.iter().take(limit) {
            let mut hit = stmt.query_row([id], hit_from_row)?;
            hit.score = score;
            hits.push(hit);
        }
        Ok(hits)
    }
}

impl LawSearch for LawIndex {
    fn warm(&self) -> AppResult<()> {
        Ok(self.warm_index()?)
    }

    fn search(&self, query: &str, filters: &LawFilters, limit: usize) -> AppResult<Vec<LawHit>> {
        Ok(self.search_index(query, filters, limit)?)
    }

    fn get_article(&self, law: LawRef, article: &str) -> AppResult<Option<ArticleView>> {
        Ok(self.article(law, article)?)
    }

    fn law_count(&self) -> AppResult<Option<u64>> {
        let conn = self.db.conn()?;
        let value: Option<String> = conn
            .query_row("SELECT value FROM meta WHERE key = 'law_count'", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(LawIndexError::from)?;
        Ok(value.and_then(|v| v.trim().parse().ok()))
    }
}
