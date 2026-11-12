//! Reference parser: finds law and provision citations in free text (Phase 3 §2.2).
//!
//! Reads `RA 9262`, `R.A. No. 9262`, `RA9262`, `Republic Act 9262`, `Sec. 5`, `Section 5`,
//! `Art. III`, `Article 3` and `Constitution` (plus the Tagalog `Seksyon`, `Artikulo`,
//! `Saligang Batas`). Pure: no IO, no allocation beyond the result.

use super::roman::from_roman;

/// A cited law.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LawRef {
    /// A Republic Act by number.
    RepublicAct(u32),
    /// The 1987 Constitution.
    Constitution,
}

/// A cited provision inside a law.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProvisionRef {
    /// `Sec. n` / `Section n`.
    Section(u32),
    /// `Art. n` / `Article n`, with `n` given in Arabic or Roman numerals.
    Article(u32),
}

/// Every citation found in one text, in order of appearance, without duplicates.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct References {
    /// Cited laws.
    pub laws: Vec<LawRef>,
    /// Cited provisions.
    pub provisions: Vec<ProvisionRef>,
}

impl References {
    /// The first cited section number, if any.
    #[must_use]
    pub fn section(&self) -> Option<u32> {
        self.provisions.iter().find_map(|p| match p {
            ProvisionRef::Section(n) => Some(*n),
            ProvisionRef::Article(_) => None,
        })
    }

    /// The first cited article number, if any.
    #[must_use]
    pub fn article(&self) -> Option<u32> {
        self.provisions.iter().find_map(|p| match p {
            ProvisionRef::Article(n) => Some(*n),
            ProvisionRef::Section(_) => None,
        })
    }
}

/// Largest law, section or article number accepted; longer digit runs are not citations.
const MAX_NUMBER: u32 = 99_999;

/// Parses every law and provision citation in `text`.
#[must_use]
pub fn parse_references(text: &str) -> References {
    let tokens = tokenize(text);
    let mut refs = References::default();
    let mut i = 0;
    while i < tokens.len() {
        i += match_at(&tokens, i, &mut refs).unwrap_or(1);
    }
    refs
}

/// Lower-cased alphanumeric runs; every other character is a separator.
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Tries every citation form at `i`; returns how many tokens it consumed.
fn match_at(tokens: &[String], i: usize, refs: &mut References) -> Option<usize> {
    let t = tokens.get(i)?.as_str();
    if t == "constitution" || t == "konstitusyon" {
        push(&mut refs.laws, LawRef::Constitution);
        return Some(1);
    }
    if t == "saligang" && tokens.get(i + 1).is_some_and(|n| n == "batas") {
        push(&mut refs.laws, LawRef::Constitution);
        return Some(2);
    }
    if let Some(n) = t.strip_prefix("ra").and_then(arabic) {
        push(&mut refs.laws, LawRef::RepublicAct(n));
        return Some(1);
    }
    let law_head = match (t, tokens.get(i + 1).map(String::as_str)) {
        ("ra", _) => Some(1),
        ("r", Some("a")) | ("republic", Some("act")) => Some(2),
        _ => None,
    };
    if let Some(head) = law_head {
        let (n, used) = number_after(tokens, i + head, arabic)?;
        push(&mut refs.laws, LawRef::RepublicAct(n));
        return Some(head + used);
    }
    if matches!(t, "sec" | "secs" | "section" | "seksyon") {
        let (n, used) = number_after(tokens, i + 1, arabic)?;
        push(&mut refs.provisions, ProvisionRef::Section(n));
        return Some(1 + used);
    }
    if matches!(t, "art" | "arts" | "article" | "artikulo") {
        let (n, used) = number_after(tokens, i + 1, arabic_or_roman)?;
        push(&mut refs.provisions, ProvisionRef::Article(n));
        return Some(1 + used);
    }
    None
}

/// Reads an optional `No.` / `Number` / `Blg.` marker, then a number; returns it and tokens used.
fn number_after(
    tokens: &[String],
    i: usize,
    read: fn(&str) -> Option<u32>,
) -> Option<(u32, usize)> {
    let marker = tokens
        .get(i)
        .is_some_and(|t| matches!(t.as_str(), "no" | "nos" | "number" | "blg"));
    let at = if marker { i + 1 } else { i };
    let n = read(tokens.get(at)?)?;
    Some((n, at - i + 1))
}

fn arabic(t: &str) -> Option<u32> {
    if t.is_empty() || t.len() > 5 || !t.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    t.parse().ok().filter(|n| (1..=MAX_NUMBER).contains(n))
}

fn arabic_or_roman(t: &str) -> Option<u32> {
    arabic(t).or_else(|| from_roman(t))
}

fn push<T: PartialEq>(list: &mut Vec<T>, item: T) {
    if !list.contains(&item) {
        list.push(item);
    }
}
