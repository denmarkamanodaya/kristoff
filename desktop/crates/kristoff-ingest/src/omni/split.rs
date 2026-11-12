//! Provision splitter for `OmniCorpus` rows, which have no newlines (Phase 7). Finds article,
//! section and rule heads glued to the previous sentence (`…violence.ART. 266. Slight…`) and keeps
//! only heads that continue the numbering, so cross-references (`under Article 36.`) and quoted
//! amendments (`…to read as follows: "SEC. 5. …`) are not taken as heads.

/// What a head starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadKind {
    /// `ART. 335.` / `Article 36.`
    Article,
    /// `SECTION 1.` / `SEC. 2.` / `Sec. 3.`
    Section,
    /// `RULE 113` (Rules of Court); sections restart under each rule.
    Rule,
}

/// One accepted head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    /// Byte offset where the head starts.
    pub at: usize,
    /// What it starts.
    pub kind: HeadKind,
    /// Its number.
    pub number: u32,
    /// Letter suffix such as `-A` (`ART. 266-A.`), empty when none.
    pub suffix: String,
}

const KEYWORDS: [(&str, HeadKind); 9] = [
    ("ARTICLE", HeadKind::Article),
    ("Article", HeadKind::Article),
    ("ART.", HeadKind::Article),
    ("Art.", HeadKind::Article),
    ("SECTION", HeadKind::Section),
    ("Section", HeadKind::Section),
    ("SEC.", HeadKind::Section),
    ("Sec.", HeadKind::Section),
    ("RULE", HeadKind::Rule),
];

/// Every head-shaped match, before the numbering filter.
#[must_use]
pub fn candidates(text: &str) -> Vec<Head> {
    let mut out = Vec::new();
    for (at, _) in text.char_indices() {
        let found = KEYWORDS
            .iter()
            .filter(|(kw, _)| after_boundary(text, at, kw))
            .find_map(|(kw, kind)| head_at(text, at, kw, *kind));
        out.extend(found);
    }
    out
}

/// A head follows the start, a sentence end or a quote. A letter right before it with no space
/// is a division title glued to the head (`…of PenaltiesART. 27.`, `…MarriageArticle 1.`); a
/// reference always has a space first (`under Article 36`), so a letter, digit or comma before a
/// space rejects the match. A digit glued to an all-caps head (`…3.75ART. 245.`, a table end) and a comma glued to the head (`…periods,ART. 178.`) is also a title end.
fn after_boundary(text: &str, at: usize, keyword: &str) -> bool {
    let before = &text[..at];
    let caps = !keyword.chars().any(char::is_lowercase);
    match before.chars().next_back() {
        None => true,
        Some(c) if c.is_alphabetic() => true,
        // `…3.75ART. 245.`: an all-caps head glued to the end of a table row.
        Some(c) if !c.is_whitespace() => caps || !c.is_ascii_digit(),
        Some(_) => before
            .chars()
            .rev()
            .find(|c| !c.is_whitespace())
            .is_none_or(|c| !(c.is_alphanumeric() || c == ',')),
    }
}

fn head_at(text: &str, at: usize, keyword: &str, kind: HeadKind) -> Option<Head> {
    let rest = text[at..].strip_prefix(keyword)?;
    let rest = rest.strip_prefix('.').unwrap_or(rest).trim_start();
    let digits: String = rest
        .chars()
        .take_while(char::is_ascii_digit)
        .take(5)
        .collect();
    let number: u32 = digits.parse().ok().filter(|n| *n > 0)?;
    let mut tail = &rest[digits.len()..];
    let mut suffix = String::new();
    if let Some(after) = tail.strip_prefix('-') {
        let letter = after.chars().next().filter(char::is_ascii_uppercase)?;
        suffix = format!("-{letter}");
        tail = &after[1..];
    }
    let ok = if kind == HeadKind::Rule {
        tail.trim_start()
            .chars()
            .next()
            .is_some_and(char::is_uppercase)
    } else {
        let t = tail.trim_start();
        let dotted = t
            .strip_prefix('.')
            .is_some_and(|t| !t.starts_with(|c: char| c.is_ascii_digit()));
        // `ART. 282, Denial of Registration` (a comma typo in the 1974 Labor Code text).
        let comma_title = t
            .strip_prefix(", ")
            .is_some_and(|t| t.starts_with(char::is_uppercase));
        dotted || comma_title
    };
    ok.then_some(Head {
        at,
        kind,
        number,
        suffix,
    })
}

/// Keeps heads that continue the numbering: the next number (or a suffix of the current one),
/// or a forward jump only when no later candidate supplies the next number first.
#[must_use]
pub fn accepted(cands: &[Head]) -> Vec<Head> {
    let mut out: Vec<Head> = Vec::new();
    let (mut last_art, mut last_sec, mut last_rule) = (0u32, 0u32, 0u32);
    for (i, h) in cands.iter().enumerate() {
        let last = match h.kind {
            HeadKind::Article => last_art,
            HeadKind::Section => last_sec,
            HeadKind::Rule => last_rule,
        };
        let next = h.number == last + 1 && h.suffix.is_empty();
        let suffix_of_last = h.number == last && !h.suffix.is_empty();
        let jump = h.number > last + 1
            && !cands[i + 1..]
                .iter()
                .any(|c| c.kind == h.kind && c.number == last + 1);
        if !(next || suffix_of_last || jump) {
            continue;
        }
        match h.kind {
            HeadKind::Article => last_art = h.number,
            HeadKind::Section => last_sec = h.number,
            HeadKind::Rule => (last_rule, last_sec) = (h.number, 0),
        }
        out.push(h.clone());
    }
    out
}

/// One provision cut from a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provision {
    /// `ART. 335`, `SEC. 2`, `RULE 113 SEC. 5`, or `PREAMBLE` for text before the first head.
    pub label: String,
    /// The provision text, starting with its head.
    pub text: String,
    /// The rule a Rules of Court section sits under.
    pub rule: Option<u32>,
}

/// Splits `text` at its accepted heads. Rule heads only set the context of the sections under
/// them; their own text (the rule title) goes to the preamble or the previous provision.
#[must_use]
pub fn split(text: &str) -> Vec<Provision> {
    let heads: Vec<Head> = accepted(&candidates(text));
    let primary = primary_kind(&heads);
    let mut out = Vec::new();
    let cuts: Vec<&Head> = heads.iter().filter(|h| h.kind == primary).collect();
    let first = cuts.first().map_or(text.len(), |h| h.at);
    if text[..first].trim().len() > 40 {
        let text = text[..first].trim().to_owned();
        out.push(Provision {
            label: "PREAMBLE".to_owned(),
            text,
            rule: None,
        });
    }
    for (i, h) in cuts.iter().enumerate() {
        let end = cuts.get(i + 1).map_or(text.len(), |n| n.at);
        let rule = heads
            .iter()
            .rfind(|r| r.kind == HeadKind::Rule && r.at < h.at);
        let text = text[h.at..end].trim().to_owned();
        out.push(Provision {
            label: label(h, rule),
            text,
            rule: rule.map(|r| r.number),
        });
    }
    out
}

/// The kind a document is cut on: the more frequent of articles and sections. The other kind is
/// a division title inside it (the Family Code's "Section 1. General Provisions").
fn primary_kind(heads: &[Head]) -> HeadKind {
    let count = |k: HeadKind| heads.iter().filter(|h| h.kind == k).count();
    if count(HeadKind::Article) > count(HeadKind::Section) {
        HeadKind::Article
    } else {
        HeadKind::Section
    }
}

fn label(h: &Head, rule: Option<&Head>) -> String {
    let base = match h.kind {
        HeadKind::Article => format!("ART. {}{}", h.number, h.suffix),
        _ => format!("SEC. {}{}", h.number, h.suffix),
    };
    match (h.kind, rule) {
        (HeadKind::Section, Some(r)) => format!("RULE {} {base}", r.number),
        _ => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(text: &str) -> Vec<String> {
        split(text).into_iter().map(|p| p.label).collect()
    }

    #[test]
    fn splits_glued_article_heads_and_skips_cross_references() {
        let text = "AN ACT REVISING THE PENAL CODE AND OTHER PENAL LAWS, BE IT ENACTED.ART. 1. Time. \
                    This Code applies.ART. 2. Application. Except as provided in Article 1. Hence.\
                    ART. 3. Definitions. As defined.";

        assert_eq!(labels(text), ["PREAMBLE", "ART. 1", "ART. 2", "ART. 3"]);
    }

    #[test]
    fn ignores_a_quoted_amendment_that_jumps_ahead() {
        let text = "SECTION 1. Title. X. SEC. 2. Section 9 of RA 1 is amended to read: \"SEC. 9. New text.\" \
                    SEC. 3. Effectivity. Upon approval.";

        assert_eq!(labels(text), ["SEC. 1", "SEC. 2", "SEC. 3"]);
    }

    #[test]
    fn accepts_suffixed_articles_and_gaps_from_repeals() {
        let text = "ART. 266. Slight. ART. 266-A. Rape. ART. 268. Next.";

        assert_eq!(labels(text), ["ART. 266", "ART. 266-A", "ART. 268"]);
    }

    #[test]
    fn splits_heads_glued_to_mixed_case_titles_but_not_spaced_references() {
        let text = "Duration of PenaltiesART. 27. Reclusion perpetua. As in Article 28. Then. \
                    Requisites of MarriageART. 28. Next.";

        assert_eq!(labels(text), ["ART. 27", "ART. 28"]);
    }

    #[test]
    fn cuts_on_the_main_kind_and_reads_a_comma_after_the_number() {
        let text = "Art. 87. Gifts.Section 1. General ProvisionsArt. 88. Community. \
                    ART. 89, Waiver. A. ART. 90. Next.";

        assert_eq!(labels(text), ["ART. 87", "ART. 88", "ART. 89", "ART. 90"]);
    }

    #[test]
    fn restarts_sections_under_each_rule() {
        let text = "RULE 1GENERAL PROVISIONSSECTION 1. Title. A. SEC. 2. Scope. B.\
                    RULE 2CAUSE OF ACTIONSECTION 1. Ordinary. C.";

        assert_eq!(
            labels(text),
            ["RULE 1 SEC. 1", "RULE 1 SEC. 2", "RULE 2 SEC. 1"]
        );
    }
}
