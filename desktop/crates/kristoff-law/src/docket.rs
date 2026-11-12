//! G.R. numbers: the one rule for reading them from free text, used by the case ingest (to key
//! decisions) and by `CaseIndex` (to find the decision a query or `get_case` names). Pure.

/// The digits of the first G.R. number in `text`: `G.R. No. 206770`, `GR 206770`,
/// `G.R. Nos. 238584-87` (first number), or a bare `206770` when `bare_digits` is set
/// (`get_case("135981")`). `None` when there is none.
#[must_use]
pub fn gr_key(text: &str, bare_digits: bool) -> Option<String> {
    let lower = text.to_lowercase();
    let mut from = 0;
    while let Some(at) = find_gr(&lower[from..]).map(|i| i + from) {
        let rest =
            lower[at..].trim_start_matches(|c: char| c.is_alphabetic() || c == '.' || c == ' ');
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if (4..=7).contains(&digits.len()) {
            return Some(digits);
        }
        from = at + 1;
    }
    if bare_digits {
        let t = text.trim();
        if (4..=7).contains(&t.len()) && t.chars().all(|c| c.is_ascii_digit()) {
            return Some(t.to_owned());
        }
    }
    None
}

/// Byte offset of the next `g.r.` / `gr` token (start of word) in lower-cased text; always a
/// char boundary, so non-ASCII text (mojibake included) is safe.
fn find_gr(lower: &str) -> Option<usize> {
    let mut prev: Option<char> = None;
    for (i, c) in lower.char_indices() {
        let start = prev.is_none_or(|p| !p.is_alphanumeric());
        let rest = &lower[i..];
        if start
            && c == 'g'
            && (rest.starts_with("g.r.") || rest.starts_with("gr ") || rest.starts_with("gr."))
        {
            return Some(i);
        }
        prev = Some(c);
    }
    None
}

/// The display form `G.R. No. <digits>`.
#[must_use]
pub fn gr_display(key: &str) -> String {
    format!("G.R. No. {key}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_common_form() {
        for text in [
            "G.R. No. 206770",
            "see GR 206770 (2014)",
            "G.R. Nos. 206770-87",
            "g.r. no.206770",
        ] {
            assert_eq!(gr_key(text, false).as_deref(), Some("206770"), "{text}");
        }
    }

    #[test]
    fn is_safe_on_non_ascii_text() {
        assert_eq!(
            gr_key(
                "MARI\u{c3}\u{b1}O \u{e2}\u{80}\u{93} G.R. No. 232974",
                false
            )
            .as_deref(),
            Some("232974")
        );
        assert_eq!(gr_key("\u{e2}\u{80}\u{93}gr\u{e2} x", false), None);
    }

    #[test]
    fn reads_bare_digits_only_when_asked_and_ignores_other_words() {
        assert_eq!(gr_key("135981", true).as_deref(), Some("135981"));
        assert_eq!(gr_key("135981", false), None);
        assert_eq!(gr_key("grounds 2019 under RA 9262", false), None);
        assert_eq!(gr_key("A.M. No. 07-3-13-SC", true), None);
    }
}
