//! Every vector in memory with the per-chunk fields search needs, and the int8 kNN scan.

use kristoff_core::domain::law::LawFilters;
use rusqlite::Row;

use crate::{
    chunk_kind::{is_heading_only, opens_provision},
    error::LawIndexError,
    quant::quantize,
    schema::kind,
    store::{dot_at, parallel_scan, vector_row},
};

/// The query that feeds [`Vectors::push_row`], column for column.
pub(super) const LOAD_SQL: &str = "SELECT v.chunk_id, c.law_id, l.number, l.year, v.scale, v.v, \
     c.sec_num, substr(c.text, 1, 200), c.art_num, c.prov, l.kind FROM vectors v \
     JOIN chunks c ON c.id = v.chunk_id JOIN laws l ON l.id = c.law_id ORDER BY v.chunk_id";

/// Fusion weight of reference material relative to law.
const REFERENCE_WEIGHT: f64 = 0.5;

/// The ids of the `n` best-scored chunks, best first (score, then id); `scored` is unsorted.
pub(super) fn top_ids(scored: &mut [Scored], n: usize) -> Vec<i64> {
    let by_score = |a: &Scored, b: &Scored| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0));
    let n = n.min(scored.len());
    if n < scored.len() {
        scored.select_nth_unstable_by(n, by_score);
    }
    let best = &mut scored[..n];
    best.sort_by(by_score);
    best.iter().map(|(id, _, _)| *id).collect()
}

/// A scored chunk: `(chunk_id, law_id, score)`.
pub(super) type Scored = (i64, i64, f32);

/// All vectors, in `chunk_id` order.
#[derive(Default)]
pub(super) struct Vectors {
    ids: Vec<i64>,
    law_ids: Vec<i64>,
    /// Provision index within the law; parts of one provision share it.
    provs: Vec<i64>,
    numbers: Vec<Option<u32>>,
    years: Vec<Option<u32>>,
    scales: Vec<f32>,
    /// False for heading-only chunks, which never enter the kNN list.
    searchable: Vec<bool>,
    /// True when the chunk opens its own section/article (`SEC. 5. ...`), not a cross-reference.
    opener: Vec<bool>,
    /// True for reference material (constitutional records, court references), which the
    /// fusion weighs at [`REFERENCE_WEIGHT`] so it never outranks law.
    reference: Vec<bool>,
    data: Vec<u8>,
}

impl Vectors {
    /// Appends one row of [`LOAD_SQL`].
    pub(super) fn push_row(&mut self, row: &Row<'_>) -> Result<(), LawIndexError> {
        let (id, scale, blob) = vector_row(row, [0, 4, 5])?;
        self.ids.push(id);
        self.law_ids.push(row.get(1)?);
        self.provs.push(row.get(9)?);
        self.numbers.push(row.get(2)?);
        self.years.push(row.get(3)?);
        self.scales.push(scale);
        let sec: Option<i64> = row.get(6)?;
        let head: String = row.get(7)?;
        self.searchable.push(!is_heading_only(sec.is_some(), &head));
        let art: Option<i64> = row.get(8)?;
        let number = sec.or(art).and_then(|n| u32::try_from(n).ok());
        self.opener
            .push(number.is_some_and(|n| opens_provision(&head, n)));
        let law_kind: String = row.get(10)?;
        self.reference
            .push(law_kind == kind::CONREC || law_kind == kind::COURTREF);
        self.data.extend_from_slice(&blob);
        Ok(())
    }

    /// Every searchable vector that passes `filters`, scored against `query`, best first.
    pub(super) fn scores(&self, query: &[f32], filters: &LawFilters) -> Vec<Scored> {
        let (q8, q_scale) = quantize(query);
        parallel_scan(self.ids.len(), |range| {
            range
                .filter(|&i| self.searchable[i] && filters.admits(self.numbers[i], self.years[i]))
                .map(|i| {
                    (
                        self.ids[i],
                        self.law_ids[i],
                        q_scale * dot_at(&self.data, i, &q8, self.scales[i]),
                    )
                })
                .collect()
        })
    }

    /// Weighs reference material down after fusion and re-sorts (score, then id).
    pub(super) fn demote_reference(&self, mut fused: Vec<(i64, f64)>) -> Vec<(i64, f64)> {
        for (id, score) in &mut fused {
            if self.ids.binary_search(id).is_ok_and(|i| self.reference[i]) {
                *score *= REFERENCE_WEIGHT;
            }
        }
        fused.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        fused
    }

    /// Whether chunk `id` opens its provision; `ids` is sorted, so this is a binary search.
    pub(super) fn is_opener(&self, id: i64) -> bool {
        self.ids.binary_search(&id).is_ok_and(|i| self.opener[i])
    }

    /// The `(law_id, prov)` provision key of chunk `id`.
    fn provision_of(&self, id: i64) -> Option<(i64, i64)> {
        self.ids
            .binary_search(&id)
            .ok()
            .map(|i| (self.law_ids[i], self.provs[i]))
    }

    /// Keeps only the best-scored part of each provision, so one long section cannot fill
    /// the result list with its parts.
    pub(super) fn one_per_provision(&self, fused: Vec<(i64, f64)>) -> Vec<(i64, f64)> {
        let mut seen = Vec::new();
        fused
            .into_iter()
            .filter(|(id, _)| match self.provision_of(*id) {
                Some(key) if seen.contains(&key) => false,
                Some(key) => {
                    seen.push(key);
                    true
                }
                None => true,
            })
            .collect()
    }
}
