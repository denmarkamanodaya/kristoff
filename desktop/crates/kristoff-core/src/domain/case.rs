//! Supreme Court decisions (docs/desktop/phases/phase-7-omnicorpus.md, "Core port CaseSearch"):
//! the value types the `CaseSearch` port speaks.

/// One decision found by a search, or the decision a `get_case` returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseHit {
    /// Stable id inside the case index (the dataset's decision id).
    pub case_id: String,
    /// The G.R. number as the decision gives it, e.g. `G.R. No. 135981`; `None` when it has none
    /// (A.M. or A.C. matters).
    pub gr_number: Option<String>,
    /// Short title, e.g. `People v. Genosa`.
    pub title: String,
    /// Promulgation date, `yyyy-mm-dd`.
    pub date: String,
    /// The ponente, when the source names one.
    pub ponente: Option<String>,
    /// The passage that matched (search) or the opening of the decision.
    pub excerpt: String,
    /// The E-Library page, when the source gives one.
    pub source_url: Option<String>,
}

/// Optional narrowing for a case search; both bounds are inclusive promulgation years.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CaseFilters {
    /// Earliest year.
    pub year_from: Option<u16>,
    /// Latest year.
    pub year_to: Option<u16>,
}

/// One decision with up to three passages: the ones matching the focus, or its opening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseView {
    /// The decision.
    pub hit: CaseHit,
    /// Passages in document order, at most three.
    pub passages: Vec<String>,
}
