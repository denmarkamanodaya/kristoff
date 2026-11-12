//! Provision parts: a long provision is stored as several chunks, each repeating the last
//! sentence of the part before it. `overlap` is the byte length of that repeated lead, so
//! [`stitch`] rebuilds the original text exactly. Shared by the ingest tool and the law index adapter.

/// One stored part of a provision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// The part's text, starting with the repeated sentence when `overlap > 0`.
    pub text: String,
    /// Bytes at the start of `text` that repeat the end of the previous part.
    pub overlap: usize,
}

/// Joins parts in order, dropping each part's repeated lead.
///
/// An `overlap` larger than its text, or not on a char boundary, keeps the whole part rather
/// than panicking (a malformed db degrades to visible duplication, not a crash).
#[must_use]
pub fn stitch<'a>(parts: impl IntoIterator<Item = (&'a str, usize)>) -> String {
    let mut out = String::new();
    for (text, overlap) in parts {
        out.push_str(text.get(overlap..).unwrap_or(text));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stitching_drops_the_repeated_sentence() {
        let parts = [("A one. B two.", 0), ("B two. C three.", 6)];

        assert_eq!(stitch(parts), "A one. B two. C three.");
    }

    #[test]
    fn a_bad_overlap_keeps_the_whole_part() {
        assert_eq!(stitch([("abc", 0), ("de", 9)]), "abcde");
    }
}
