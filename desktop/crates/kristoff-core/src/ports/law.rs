//! The law index port (docs/desktop/phases/phase-3-law-index.md §2): read-only hybrid search
//! over the bundled PH law database, used by the Phase 4 tool loop.

use crate::domain::law::{ArticleView, LawFilters, LawHit, LawRef};
use crate::error::AppResult;

/// Searches the PH law index.
pub trait LawSearch: Send + Sync {
    /// Loads what the first search needs (vectors, GPU kernels) so it is fast. Call once at
    /// start, off the UI thread; calling it again is cheap.
    ///
    /// # Errors
    /// `INTERNAL` when the index or the embedder cannot be read.
    fn warm(&self) -> AppResult<()>;
    /// Up to `limit` passages for `query`, best first, at most one per provision, all passing
    /// `filters`.
    ///
    /// # Errors
    /// `INTERNAL` when the index or the embedder fails.
    fn search(&self, query: &str, filters: &LawFilters, limit: usize) -> AppResult<Vec<LawHit>>;
    /// The provision `article` (e.g. `Sec. 5`, `Article III Section 2`) of `law`, stitched whole,
    /// with its neighbours; `None` when the law or the provision is not in the index.
    ///
    /// # Errors
    /// `INTERNAL` when the index fails.
    fn get_article(&self, law: LawRef, article: &str) -> AppResult<Option<ArticleView>>;
}
