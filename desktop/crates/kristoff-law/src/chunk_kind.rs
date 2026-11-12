//! Classifies chunks from the dataset's article splits. Pure.
//!
//! The scraper split some laws at headings (`CHAPTER IV / SECURITY OF PERSONAL INFORMATION`)
//! and at in-text cross-references (`Section 5 hereof shall be punished ...`). Both kinds carry
//! labels that look like real provisions, so search demotes them using these rules.

/// Longest text (in bytes) a heading-only chunk can have.
const HEADING_MAX_LEN: usize = 160;

/// True for a chunk that is only a division heading: no section number, short, and no
/// sentence in it.
#[must_use]
pub fn is_heading_only(has_section: bool, text: &str) -> bool {
    let text = text.trim();
    !has_section && text.len() < HEADING_MAX_LEN && !text.ends_with('.') && !text.contains(". ")
}

/// True when `text` opens provision `number` (`SEC. 5. Acts of ...`, `SECTION 1 Short Title`),
/// rather than citing it mid-sentence (`Section 5 hereof shall ...`, `Section 4(a) of ...`).
///
/// Rule: the first number in the first 40 characters is `number`, and the next non-space
/// character after it is `.`, `:`, a dash, or an upper-case letter.
#[must_use]
pub fn opens_provision(text: &str, number: u32) -> bool {
    let head: String = text.chars().take(40).collect();
    let Some(start) = head.find(|c: char| c.is_ascii_digit()) else {
        return false;
    };
    let digits: String = head[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.parse::<u32>().ok() != Some(number) {
        return false;
    }
    let after = head[start + digits.len()..].trim_start().chars().next();
    after.is_some_and(|c| matches!(c, '.' | ':' | '-' | '–' | '—') || c.is_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chapter_heading_is_heading_only() {
        assert!(is_heading_only(
            false,
            "CHAPTER IV\n\nSECURITY OF PERSONAL INFORMATION"
        ));
        assert!(!is_heading_only(
            false,
            "ART. 19. Every person must act with justice."
        ));
        assert!(!is_heading_only(true, "SEC. 9"));
    }

    #[test]
    fn a_section_opening_is_told_apart_from_a_cross_reference() {
        assert!(opens_provision("SEC. 5. Acts of Violence Against Women", 5));
        assert!(opens_provision("SECTION 1 Short Title. — This Act", 1));
        assert!(opens_provision("SEC. 4-A. Attempted Trafficking", 4));
        assert!(!opens_provision("Section 5 hereof shall be punished", 5));
        assert!(!opens_provision("Section 4(a)(5) shall be punished", 4));
        assert!(!opens_provision("SEC. 6. Penalties.", 5));
    }
}
