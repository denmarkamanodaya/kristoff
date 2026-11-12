//! The case index port (docs/desktop/phases/phase-7-omnicorpus.md): read-only search over the
//! bundled Supreme Court decisions (`phcases.db`), used by the ask flow's case tools.

use crate::domain::case::{CaseFilters, CaseHit, CaseView};
use crate::error::AppResult;

/// Searches the PH Supreme Court decisions.
pub trait CaseSearch: Send + Sync {
    /// Loads what the first search needs, so it is fast. Call once at start, off the UI thread;
    /// calling it again is cheap.
    ///
    /// # Errors
    /// `INTERNAL` when the index or the embedder cannot be read.
    fn warm(&self) -> AppResult<()>;
    /// How many decisions the index holds.
    ///
    /// # Errors
    /// `INTERNAL` when the index fails.
    fn case_count(&self) -> AppResult<u64>;
    /// Up to `limit` (at most 8) decisions for `query`, best first, at most one per decision, all
    /// passing `filters`.
    ///
    /// # Errors
    /// `INTERNAL` when the index or the embedder fails.
    fn search(&self, query: &str, filters: &CaseFilters, limit: u8) -> AppResult<Vec<CaseHit>>;
    /// The decision named by `case` (a case id or a G.R. number such as `G.R. No. 135981` or
    /// `135981`), with up to three passages matching `focus`, or its opening passages without
    /// one; `None` when the index does not hold it.
    ///
    /// # Errors
    /// `INTERNAL` when the index fails.
    fn get_case(&self, case: &str, focus: Option<&str>) -> AppResult<Option<CaseView>>;
}
