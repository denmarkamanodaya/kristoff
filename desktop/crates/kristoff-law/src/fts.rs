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
        .filter(|t| !STOPWORDS.contains(&t.trim_matches('"')))
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
            Some("\"ra\" OR \"9262\" OR \"near\" OR \"x\" OR \"title\" OR \"foo\"")
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
    fn returns_none_for_text_without_words() {
        assert_eq!(match_expression(" -- ** "), None);
    }
}
