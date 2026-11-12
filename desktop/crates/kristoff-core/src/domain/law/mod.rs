//! The PH law index domain (docs/desktop/phases/phase-3-law-index.md): citation parsing,
//! provision stitching and the value types the `LawSearch` port speaks.

pub mod provision;
pub mod reference;
pub mod roman;
pub mod tools;

pub use reference::LawRef;

/// The law a hit belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawSummary {
    /// Display identifier, e.g. `Republic Act No. 9262` or `1987 Constitution`.
    pub identifier: String,
    /// Long title.
    pub title: String,
    /// Republic Act number; `None` for the Constitution.
    pub number: Option<u32>,
    /// Approval year when the source gives one.
    pub year: Option<u32>,
}

/// One law passage: a search hit (one part of a provision) or a whole provision.
#[derive(Debug, Clone, PartialEq)]
pub struct LawHit {
    /// Chunk id inside this index build; stable only within one build.
    pub chunk_id: i64,
    /// The owning law.
    pub law: LawSummary,
    /// Provision label as the source gives it, e.g. `SEC. 5` or `ARTICLE III SECTION 2`.
    pub label: String,
    /// Passage text.
    pub text: String,
    /// Source page.
    pub source_url: String,
    /// Source law status, e.g. `current`.
    pub status: String,
    /// The source text looked cut off mid-sentence.
    pub truncated: bool,
    /// Index of this part within its provision, from 0; whole provisions carry 0.
    pub part: u32,
    /// Fused relevance score, higher is better; 0 for `get_article` results.
    pub score: f64,
}

/// Optional narrowing for a law search; every field is inclusive.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LawFilters {
    /// Earliest approval year.
    pub year_from: Option<u32>,
    /// Latest approval year.
    pub year_to: Option<u32>,
    /// Only this Republic Act number.
    pub law_number: Option<u32>,
}

impl LawFilters {
    /// True when a law with `number` and `year` passes every set filter. A year bound rejects
    /// laws whose year is unknown.
    #[must_use]
    pub fn admits(&self, number: Option<u32>, year: Option<u32>) -> bool {
        let within = |bound: Option<u32>, ok: fn(u32, u32) -> bool| {
            bound.is_none_or(|b| year.is_some_and(|y| ok(y, b)))
        };
        self.law_number.is_none_or(|n| number == Some(n))
            && within(self.year_from, |y, b| y >= b)
            && within(self.year_to, |y, b| y <= b)
    }
}

/// A whole provision with its neighbouring provisions in the same law.
#[derive(Debug, Clone, PartialEq)]
pub struct ArticleView {
    /// The requested provision, every part stitched in order.
    pub hit: LawHit,
    /// The provision before it, stitched.
    pub previous: Option<LawHit>,
    /// The provision after it, stitched.
    pub next: Option<LawHit>,
}

#[cfg(test)]
mod tests {
    use super::LawFilters;

    #[test]
    fn year_bounds_reject_unknown_years_and_admit_the_range() {
        let f = LawFilters {
            year_from: Some(2010),
            year_to: Some(2012),
            law_number: None,
        };

        assert!(f.admits(Some(1), Some(2011)));
        assert!(!f.admits(Some(1), Some(2013)));
        assert!(!f.admits(Some(1), None));
        assert!(LawFilters::default().admits(None, None));
    }

    #[test]
    fn a_law_number_filter_admits_only_that_act() {
        let f = LawFilters {
            law_number: Some(9262),
            ..LawFilters::default()
        };

        assert!(f.admits(Some(9262), None));
        assert!(!f.admits(Some(10173), None));
        assert!(!f.admits(None, Some(1987)));
    }
}
