//! The part-splitting rules for large PDFs (docs/contract.md "v2 Large PDFs"). A document is sent
//! whole only when it has at most 100 pages and 30 MiB; otherwise it becomes parts of at most 100
//! pages, and a part that comes out larger than 30 MiB is halved until it fits; a single page that
//! is still too large cannot be sent. Ported from `backend/src/domain/docs/plan-parts/v1.ts`.

use crate::limits::{MAX_PARTS, PART_MAX_BYTES, PART_MAX_PAGES};

/// A 1-based inclusive page range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRange {
    /// First page, 1-based.
    pub start_page: u32,
    /// Last page, inclusive.
    pub end_page: u32,
}

impl PageRange {
    /// Pages in the range.
    #[must_use]
    pub fn len(self) -> u32 {
        self.end_page.saturating_sub(self.start_page) + 1
    }

    /// Never true for a range made here; present so `len` reads naturally.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.end_page < self.start_page
    }
}

/// The limits one split respects (production: [`SplitLimits::CLAUDE`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitLimits {
    /// Most pages per part, and per document sent whole.
    pub max_pages: u32,
    /// Most bytes per part, and per document sent whole.
    pub max_bytes: u64,
    /// Most parts one document may become.
    pub max_parts: usize,
}

impl SplitLimits {
    /// 100 pages, 31,457,280 bytes, 200 parts.
    pub const CLAUDE: Self = Self {
        max_pages: PART_MAX_PAGES,
        max_bytes: PART_MAX_BYTES,
        max_parts: MAX_PARTS,
    };

    /// Whether a document must be split.
    #[must_use]
    pub fn needs_parts(self, page_count: u32, size: u64) -> bool {
        page_count > self.max_pages || size > self.max_bytes
    }

    /// The first cut: consecutive ranges of at most `max_pages` covering every page.
    #[must_use]
    pub fn initial_ranges(self, page_count: u32) -> Vec<PageRange> {
        let step = self.max_pages.max(1);
        (0..page_count.div_ceil(step))
            .map(|i| PageRange {
                start_page: i * step + 1,
                end_page: page_count.min((i + 1) * step),
            })
            .collect()
    }

    /// Whether a produced part fits the byte limit.
    #[must_use]
    pub fn fits(self, size: u64) -> bool {
        size <= self.max_bytes
    }
}

/// Splits an oversized range into two halves in order (the extra page goes first); `None` for a
/// single page.
#[must_use]
pub fn halve_range(range: PageRange) -> Option<(PageRange, PageRange)> {
    if range.start_page >= range.end_page {
        return None;
    }
    let mid = u32::midpoint(range.start_page, range.end_page);
    Some((
        PageRange {
            start_page: range.start_page,
            end_page: mid,
        },
        PageRange {
            start_page: mid + 1,
            end_page: range.end_page,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const L: SplitLimits = SplitLimits::CLAUDE;

    #[test]
    fn a_document_at_the_limits_is_sent_whole() {
        assert!(!L.needs_parts(100, 31_457_280));
        assert!(L.needs_parts(101, 1));
        assert!(L.needs_parts(1, 31_457_281));
    }

    #[test]
    fn a_single_page_cannot_be_halved_and_an_odd_range_puts_the_extra_page_first() {
        assert_eq!(
            halve_range(PageRange {
                start_page: 7,
                end_page: 7
            }),
            None
        );
        let halves = halve_range(PageRange {
            start_page: 1,
            end_page: 5,
        })
        .unwrap();
        assert_eq!(
            halves.0,
            PageRange {
                start_page: 1,
                end_page: 3
            }
        );
        assert_eq!(
            halves.1,
            PageRange {
                start_page: 4,
                end_page: 5
            }
        );
    }

    #[test]
    fn exactly_the_byte_limit_fits() {
        assert!(L.fits(31_457_280));
        assert!(!L.fits(31_457_281));
    }

    proptest! {
        #[test]
        fn initial_ranges_cover_every_page_once_in_order(n in 1u32..5000) {
            let mut next = 1;
            for r in L.initial_ranges(n) {
                prop_assert_eq!(r.start_page, next);
                prop_assert!(!r.is_empty() && r.len() <= 100);
                next = r.end_page + 1;
            }
            prop_assert_eq!(next, n + 1);
        }

        #[test]
        fn halves_cover_the_range_exactly(start in 1u32..1000, len in 2u32..200) {
            let r = PageRange { start_page: start, end_page: start + len - 1 };
            let (a, b) = halve_range(r).unwrap();
            prop_assert_eq!((a.start_page, a.end_page + 1, b.end_page), (r.start_page, b.start_page, r.end_page));
            prop_assert!(a.len() >= b.len() && a.len() - b.len() <= 1);
        }
    }
}
