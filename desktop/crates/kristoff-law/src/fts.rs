//! Builds safe FTS5 MATCH strings from user text (Phase 3 §2.1). Pure.

/// Most terms one query may carry.
const MAX_TERMS: usize = 32;

/// English and Tagalog function words dropped from MATCH while any other term remains: they
/// match most chunks, slow BM25 down and add no ranking signal.
const STOPWORDS: [&str; 52] = [
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "in", "is", "it", "of", "on",
    "or", "that", "the", "this", "to", "what", "which", "with", "under", "ako", "ang", "ano", "ay",
    "ba", "bang", "din", "ito", "ka", "kay", "ko", "kung", "lang", "mga", "mo", "na", "nang", "ng",
    "ni", "niya", "para", "po", "pwede", "rin", "sa", "si", "yung", "dapat",
];

/// Citation words: they occur in most of the ~400k chunk prefixes, and the reference parser's
/// exact-hit list already handles the citation they belong to.
const CITATION_WORDS: [&str; 9] = [
    "ra", "sec", "section", "art", "article", "no", "nos", "republic", "act",
];

/// True for a term too common to search on while others remain: a function word, a citation
/// word, a single character, or a number of at most three digits (`5`, `113`).
fn is_noise(term: &str) -> bool {
    let short_number = term.len() <= 3 && term.chars().all(|c| c.is_ascii_digit());
    STOPWORDS.contains(&term)
        || CITATION_WORDS.contains(&term)
        || term.chars().count() == 1
        || short_number
}

/// The same terms joined with implicit AND (every term must occur), for a first, narrower pass
/// over large indexes; `None` when there is no searchable word.
#[must_use]
pub fn match_all_expression(text: &str) -> Option<String> {
    match_expression(text).map(|e| e.replace(" OR ", " "))
}

/// Runs `pass` with the all-terms expression, then with the any-term one when the first gives
/// fewer than `min` results; returns the last pass's results. On large indexes the narrow pass is
/// fast, and ranking every any-term match by BM25 is the slow part.
///
/// # Errors
/// The first error a pass returns.
pub fn two_pass<T, E>(
    query: &str,
    min: usize,
    mut pass: impl FnMut(&str) -> Result<Vec<T>, E>,
) -> Result<Vec<T>, E> {
    let mut out = Vec::new();
    for expr in [match_all_expression(query), match_expression(query)]
        .into_iter()
        .flatten()
    {
        out = pass(&expr)?;
        if out.len() >= min {
            break;
        }
    }
    Ok(out)
}

/// Turns free text into `"term1" OR "term2" ...`, or `None` when it has no searchable word.
///
/// Only alphanumeric runs survive, each wrapped in double quotes, so no FTS5 operator, column
/// filter or quote from user text ever reaches MATCH.
#[must_use]
pub fn match_expression(text: &str) -> Option<String> {
    let mut terms: Vec<String> = Vec::new();
    for word in text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        let quoted = format!("\"{}\"", word.to_lowercase());
        if !terms.contains(&quoted) {
            terms.push(quoted);
        }
        if terms.len() == MAX_TERMS {
            break;
        }
    }
    let content: Vec<String> = terms
        .iter()
        .filter(|t| !is_noise(t.trim_matches('"')))
        .cloned()
        .collect();
    let kept = if content.is_empty() { terms } else { content };
    (!kept.is_empty()).then(|| kept.join(" OR "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_each_word_and_drops_fts_syntax() {
        let got = match_expression("RA 9262 NEAR(\"x\") title:foo*");

        assert_eq!(
            got.as_deref(),
            // `ra` (a citation word) and `x` (one character) are dropped as noise; see `is_noise`.
            Some("\"9262\" OR \"near\" OR \"title\" OR \"foo\"")
        );
    }

    #[test]
    fn drops_function_words_while_content_words_remain() {
        assert_eq!(
            match_expression("ano ang penalty ng estafa").as_deref(),
            Some("\"penalty\" OR \"estafa\"")
        );
        assert_eq!(
            match_expression("the of").as_deref(),
            Some("\"the\" OR \"of\"")
        );
    }

    #[test]
    fn the_all_terms_form_drops_the_or() {
        assert_eq!(
            match_all_expression("chain of custody").as_deref(),
            Some("\"chain\" \"custody\"")
        );
    }

    #[test]
    fn drops_citation_words_and_short_numbers_while_content_words_remain() {
        assert_eq!(
            match_expression("RA 9262 Sec. 5").as_deref(),
            Some("\"9262\"")
        );
        assert_eq!(
            match_expression("Rule 113 Section 5 arrest").as_deref(),
            Some("\"rule\" OR \"arrest\"")
        );
        assert_eq!(
            match_expression("Sec. 5").as_deref(),
            Some("\"sec\" OR \"5\"")
        );
    }

    #[test]
    fn returns_none_for_text_without_words() {
        assert_eq!(match_expression(" -- ** "), None);
    }
}
