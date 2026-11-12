//! The uncited statutory claim check (phase-5 §1). A statutory claim is a sentence that names a law
//! or provision (`RA 9262`, `Republic Act`, `Sec. 5`, `Section 5`, `Art. III`, `Article 2176`) and
//! is not a heading or label line. It has a source when it carries a citation marker ` [n]` (law or
//! doc), when the next sentence starts with one (a marker is stored where the cited passage
//! starts), or when its paragraph or list cites the same law and provision. Anything else is
//! uncited.

use kristoff_core::domain::law::reference::{LawRef, References, parse_references};

/// Words after which a period does not end a sentence.
const ABBREVIATIONS: [&str; 16] = [
    "sec", "secs", "art", "arts", "no", "nos", "ra", "r.a", "rep", "vs", "v", "e.g", "i.e", "inc",
    "par", "pars",
];

fn ends_with_abbreviation(before: &str) -> bool {
    let word = before
        .rsplit(|c: char| c.is_whitespace() || c == '(')
        .next()
        .unwrap_or("");
    let word = word
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    ABBREVIATIONS.contains(&word.as_str())
        || (word.len() == 1 && word.chars().all(char::is_alphabetic))
}

/// The text cut into sentences: at line breaks, and after `.`, `!` or `?` followed by a space,
/// except after an abbreviation such as `Sec.` or `R.A.`.
pub fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut start = 0;
        let bytes = line.as_bytes();
        for (i, b) in bytes.iter().enumerate() {
            let at_break =
                matches!(b, b'.' | b'!' | b'?') && bytes.get(i + 1).is_some_and(|n| *n == b' ');
            if at_break && !ends_with_abbreviation(&line[start..i]) {
                out.push(line[start..=i].trim().to_owned());
                start = i + 1;
            }
        }
        out.push(line[start..].trim().to_owned());
    }
    out.into_iter()
        .filter(|s| s.chars().any(char::is_alphanumeric))
        .collect()
}

fn words(sentence: &str) -> Vec<String> {
    sentence
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | ':' | '(' | ')' | '*' | '"'))
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

fn is_number_or_roman(word: &str) -> bool {
    let w = word.trim_end_matches(['.', ',']);
    w.starts_with(|c: char| c.is_ascii_digit())
        || (!w.is_empty() && w.chars().all(|c| matches!(c, 'I' | 'V' | 'X' | 'L')))
}

/// Whether the sentence names a statute or a provision.
pub fn names_a_law(sentence: &str) -> bool {
    let w = words(sentence);
    w.iter().enumerate().any(|(i, word)| {
        let bare = word.trim_end_matches('.').to_lowercase();
        let next = w.get(i + 1).map_or("", String::as_str);
        let after = w.get(i + 2).map_or("", String::as_str);
        let ra = matches!(bare.as_str(), "ra" | "r.a")
            && (is_number_or_roman(next)
                || (next.trim_end_matches('.') == "No" && is_number_or_roman(after)));
        let glued = bare.starts_with("ra") && bare[2..].starts_with(|c: char| c.is_ascii_digit());
        let republic_act = bare == "republic" && next.to_lowercase().starts_with("act");
        let provision = matches!(
            bare.as_str(),
            "sec" | "secs" | "section" | "sections" | "art" | "arts" | "article" | "articles"
        ) && is_number_or_roman(next);
        ra || glued || republic_act || provision
    })
}

/// The citation numbers marked ` [n]` in a sentence.
pub fn markers(sentence: &str) -> Vec<u32> {
    sentence
        .split('[')
        .skip(1)
        .filter_map(|rest| rest.split(']').next())
        .filter_map(|n| n.trim().parse().ok())
        .collect()
}

/// The citation number a sentence starts with, if any (`[3]Victim-survivors ...`).
fn leading_marker(sentence: &str) -> Option<u32> {
    let rest = sentence
        .trim_start_matches(['-', '*', ' '])
        .strip_prefix('[')?;
    rest.split(']').next()?.trim().parse().ok()
}

/// One citation of the answer: its marker number and, for a law citation, the provision its
/// title (`<identifier> · <label>`) names.
#[derive(Debug, Clone)]
pub struct Cite {
    pub n: u32,
    pub provision: Option<References>,
}

impl Cite {
    /// A citation from the stored message's JSON (`n`, `kind`, `docTitle`).
    pub fn from_json(c: &serde_json::Value) -> Option<Self> {
        let n = u32::try_from(c["n"].as_u64()?).ok()?;
        let provision =
            (c["kind"] == "law").then(|| parse_references(c["docTitle"].as_str().unwrap_or("")));
        Some(Self { n, provision })
    }
}

/// Most words a label line has (`Parusa (Sec. 6)`, `Employer liability (Art. 2180)`).
const LABEL_MAX_WORDS: usize = 8;

/// Headings and label lines introduce content and claim nothing: a `#` heading, a line that is
/// all bold (`**Art. 45.**`), or a short line with no sentence-ending punctuation and no citation.
pub fn is_heading_or_label(line: &str) -> bool {
    let line = line.trim();
    let bare = line
        .trim_start_matches(['-', '*', ' '])
        .trim_end_matches([':', '*', ' ']);
    let bold = line.trim_start_matches("- ").trim_end_matches(':').trim();
    let inner = bold.strip_prefix("**").and_then(|b| b.strip_suffix("**"));
    let all_bold = inner.is_some_and(|i| !i.contains("**"));
    let short_label = !bare.ends_with(['.', '!', '?'])
        && sentences(bare).len() <= 1
        && !bare.contains('[')
        && bare.split_whitespace().count() <= LABEL_MAX_WORDS;
    line.starts_with('#') || all_bold || short_label
}

fn is_list_item(paragraph: &str) -> bool {
    let t = paragraph.trim_start();
    t.starts_with("- ")
        || t.starts_with("* ")
        || t.split_once(". ")
            .is_some_and(|(n, _)| n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty())
}

/// Paragraphs (split at blank lines) grouped so a list, and the paragraph that leads into it,
/// form one group: `(group, paragraph)`.
fn grouped_paragraphs(text: &str) -> Vec<(usize, String)> {
    let paragraphs: Vec<String> = text
        .split("\n\n")
        .map(str::to_owned)
        .filter(|p| !p.trim().is_empty())
        .collect();
    let mut out = Vec::new();
    let mut group = 0;
    for (i, p) in paragraphs.iter().enumerate() {
        let joins_previous = i > 0
            && is_list_item(p)
            && (is_list_item(&paragraphs[i - 1])
                || !paragraphs[i - 1].trim_start().starts_with('#'));
        if i > 0 && !joins_previous {
            group += 1;
        }
        out.push((group, p.clone()));
    }
    out
}

/// Whether a sentence's provision (law + section or article) is the one a citation names. A
/// sentence that names no law takes the last law named earlier in the answer.
fn same_provision(named: &References, context_law: Option<LawRef>, cited: &References) -> bool {
    let law = named.laws.first().copied().or(context_law);
    let law_ok = law.is_none_or(|l| cited.laws.contains(&l));
    let provision_ok = match named.section() {
        Some(sec) => cited.section() == Some(sec),
        None => {
            named.article().is_some()
                && cited.article() == named.article()
                && cited.section().is_none()
        }
    };
    law_ok && provision_ok
}

/// One sentence with where it sits; a heading or label line stays whole.
struct Placed {
    text: String,
    group: usize,
    label: bool,
}

fn placed_sentences(text: &str) -> Vec<Placed> {
    let mut out = Vec::new();
    for (group, paragraph) in grouped_paragraphs(text) {
        for line in paragraph.lines() {
            if is_heading_or_label(line) {
                out.push(Placed {
                    text: line.trim().to_owned(),
                    group,
                    label: true,
                });
            } else {
                let parts = sentences(line).into_iter();
                out.extend(parts.map(|text| Placed {
                    text,
                    group,
                    label: false,
                }));
            }
        }
    }
    out
}

/// The sentences that state statutory content without a source: they name a law or provision,
/// are not a heading or label, carry no citation marker (law or doc), are not followed right away
/// by a cited passage, and the same paragraph or list does not cite that provision.
pub fn uncited_statutory_sentences(text: &str, citations: &[Cite]) -> Vec<String> {
    let all = placed_sentences(text);
    let known = |n: u32| citations.iter().any(|c| c.n == n);
    let law_cites_in = |group: usize| -> Vec<&References> {
        let ns: Vec<u32> = all
            .iter()
            .filter(|s| s.group == group)
            .flat_map(|s| markers(&s.text))
            .collect();
        citations
            .iter()
            .filter(|c| ns.contains(&c.n))
            .filter_map(|c| c.provision.as_ref())
            .collect()
    };
    let mut context_law = None;
    let mut uncited = Vec::new();
    for (i, s) in all.iter().enumerate() {
        let named = parse_references(&s.text);
        let law_before = context_law;
        context_law = named.laws.first().copied().or(context_law);
        if s.label || !names_a_law(&s.text) {
            continue;
        }
        let marked = markers(&s.text).into_iter().any(known);
        let followed = all
            .get(i + 1)
            .and_then(|n| leading_marker(&n.text))
            .is_some_and(known);
        let in_group = law_cites_in(s.group)
            .iter()
            .any(|c| same_provision(&named, law_before, c));
        if !(marked || followed || in_group) {
            uncited.push(s.text.clone());
        }
    }
    uncited
}
