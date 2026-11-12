//! Every decision-passage vector in memory (one per 480-token passage, D-20) and the parallel
//! int8 scan that finds the best passage per decision.

use rusqlite::Row;

use crate::error::LawIndexError;
use crate::quant::{dot_i8, quantize};
use crate::store::{DIMS, vector_row};

/// Threads the scan splits over (about 550k vectors).
const SCAN_THREADS: usize = 4;

/// The query that feeds [`PassageVectors::push_row`].
pub(super) const LOAD_SQL: &str = "SELECT v.passage_id, p.case_rowid, c.year, v.scale, v.v \
     FROM passage_vectors v JOIN passages p ON p.id = v.passage_id JOIN cases c ON c.id = p.case_rowid \
     ORDER BY v.passage_id";

/// All passage vectors.
#[derive(Default)]
pub(super) struct PassageVectors {
    passages: Vec<i64>,
    cases: Vec<i64>,
    years: Vec<u16>,
    scales: Vec<f32>,
    data: Vec<u8>,
}

impl PassageVectors {
    /// Appends one passage vector, a row of [`LOAD_SQL`].
    pub(super) fn push_row(&mut self, row: &Row<'_>) -> Result<(), LawIndexError> {
        let (id, scale, blob) = vector_row(row, [0, 3, 4])?;
        self.passages.push(id);
        self.cases.push(row.get(1)?);
        self.years.push(row.get(2)?);
        self.scales.push(scale);
        self.data.extend_from_slice(&blob);
        Ok(())
    }

    /// How many vectors are loaded.
    pub(super) fn len(&self) -> usize {
        self.passages.len()
    }

    /// The best `(case_rowid, passage_id)` per decision for `query`, best first, at most `take`,
    /// keeping only decisions whose year passes `admit`.
    pub(super) fn best_cases(
        &self,
        query: &[f32],
        admit: impl Fn(u16) -> bool + Sync,
        take: usize,
    ) -> Vec<(i64, i64)> {
        let (q8, _) = quantize(query);
        let per = self.len().div_ceil(SCAN_THREADS).max(1);
        let mut scored: Vec<(i32, usize)> = std::thread::scope(|s| {
            let handles: Vec<_> = (0..SCAN_THREADS)
                .map(|t| {
                    let (q8, admit) = (&q8, &admit);
                    s.spawn(move || self.scan(q8, admit, t * per..((t + 1) * per).min(self.len())))
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap_or_default())
                .collect()
        });
        scored.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut seen = std::collections::HashSet::new();
        scored
            .into_iter()
            .filter(|(_, i)| seen.insert(self.cases[*i]))
            .take(take)
            .map(|(_, i)| (self.cases[i], self.passages[i]))
            .collect()
    }

    /// Scores the vectors in `range`; the per-vector scale is folded into an integer by fixed-point so
    /// the merge can sort integers. Keeps the best 512 of the range.
    fn scan(
        &self,
        q8: &[u8],
        admit: &(impl Fn(u16) -> bool + Sync),
        range: std::ops::Range<usize>,
    ) -> Vec<(i32, usize)> {
        let mut out: Vec<(i32, usize)> = range
            .filter(|&i| admit(self.years[i]))
            .map(|i| {
                let raw = dot_i8(q8, &self.data[i * DIMS..(i + 1) * DIMS]);
                // raw * scale is at most ~127 * 127 * 384 / 127, so scaling by 2^16 fits in i32.
                #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                let fixed = (raw as f32 * self.scales[i] * 65_536.0) as i32;
                (fixed, i)
            })
            .collect();
        out.sort_unstable_by_key(|x| std::cmp::Reverse(x.0));
        out.truncate(512);
        out
    }
}
