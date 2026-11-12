//! A `LawSearch` that passes every call to the real index and records it: the tool, its query
//! and the provisions returned, in rank order (what the model saw).

use kristoff_core::domain::law::{ArticleView, LawFilters, LawHit, LawRef};
use kristoff_core::error::AppResult;
use kristoff_core::ports::LawSearch;
use std::sync::{Arc, Mutex, PoisonError};

/// One law tool call as the index saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawCall {
    /// `search_ph_laws` or `get_law_article`.
    pub tool: &'static str,
    /// The search text, or `<law> <article>` for a look-up.
    pub query: String,
    /// `(identifier, label)` of each provision returned, best first.
    pub provisions: Vec<(String, String)>,
}

/// The calls recorded since the last [`Recorded::take`].
#[derive(Debug, Clone, Default)]
pub struct Recorded(Arc<Mutex<Vec<LawCall>>>);

impl Recorded {
    /// Removes and returns the recorded calls.
    pub fn take(&self) -> Vec<LawCall> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }

    fn push(&self, call: LawCall) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call);
    }
}

/// Wraps the real index.
pub struct RecordingLaw {
    pub inner: Box<dyn LawSearch>,
    pub log: Recorded,
}

fn named(hit: &LawHit) -> (String, String) {
    (hit.law.identifier.clone(), hit.label.clone())
}

impl LawSearch for RecordingLaw {
    fn warm(&self) -> AppResult<()> {
        self.inner.warm()
    }

    fn search(&self, query: &str, filters: &LawFilters, limit: usize) -> AppResult<Vec<LawHit>> {
        let hits = self.inner.search(query, filters, limit)?;
        let provisions = hits.iter().map(named).collect();
        self.log.push(LawCall {
            tool: "search_ph_laws",
            query: query.to_owned(),
            provisions,
        });
        Ok(hits)
    }

    fn get_article(&self, law: LawRef, article: &str) -> AppResult<Option<ArticleView>> {
        let view = self.inner.get_article(law, article)?;
        let provisions = view.as_ref().map_or_else(Vec::new, |v| {
            [Some(&v.hit), v.previous.as_ref(), v.next.as_ref()]
                .into_iter()
                .flatten()
                .map(named)
                .collect()
        });
        let query = format!("{law:?} {article}");
        self.log.push(LawCall {
            tool: "get_law_article",
            query,
            provisions,
        });
        Ok(view)
    }

    fn law_count(&self) -> AppResult<Option<u64>> {
        self.inner.law_count()
    }
}
