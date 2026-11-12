//! Reciprocal rank fusion (Phase 3 §2.3). Pure.

/// RRF constant `K` from the plan.
pub const RRF_K: f64 = 60.0;

/// One ranked candidate list and the weight its ranks carry.
pub struct RankedList<'a> {
    /// Chunk ids, best first.
    pub ids: &'a [i64],
    /// Multiplier on `1 / (K + rank)`.
    pub weight: f64,
}

/// Fuses ranked lists: `score(d) = Σ weight / (K + rank)`, rank starting at 1.
///
/// Returns `(id, score)` best first; ties break on the smaller id so the order is total.
#[must_use]
pub fn fuse(lists: &[RankedList<'_>]) -> Vec<(i64, f64)> {
    let mut scores: Vec<(i64, f64)> = Vec::new();
    for list in lists {
        for (rank, id) in (1u32..).zip(list.ids) {
            let add = list.weight / (RRF_K + f64::from(rank));
            match scores.iter_mut().find(|(seen, _)| seen == id) {
                Some((_, s)) => *s += add,
                None => scores.push((*id, add)),
            }
        }
    }
    scores.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    scores
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_an_id_found_by_both_lists_above_ids_found_by_one() {
        let fts = [1, 2, 3];
        let knn = [4, 3, 5];

        let fused = fuse(&[
            RankedList {
                ids: &fts,
                weight: 1.0,
            },
            RankedList {
                ids: &knn,
                weight: 1.0,
            },
        ]);

        assert_eq!(fused[0].0, 3);
    }

    #[test]
    fn breaks_score_ties_on_the_smaller_id() {
        let a = [7];
        let b = [5];

        let fused = fuse(&[
            RankedList {
                ids: &a,
                weight: 1.0,
            },
            RankedList {
                ids: &b,
                weight: 1.0,
            },
        ]);

        assert_eq!(fused.iter().map(|x| x.0).collect::<Vec<_>>(), vec![5, 7]);
    }
}
