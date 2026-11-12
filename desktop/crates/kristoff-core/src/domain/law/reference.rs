//! Reference parser: finds law and provision citations in free text (Phase 3 §2.2, Phase 7).
//!
//! Reads `RA 9262`, `R.A. No. 9262`, `RA9262`, `Republic Act 9262`, the older issuances (`Act No.
//! 3815`, `C.A. No. 141`, `B.P. Blg. 22`, `P.D. No. 442`, `E.O. 209`, `LOI 1`), the codes by name
//! (`Revised Penal Code`/`RPC`, `Family Code`, `Labor Code`, `Rules of Court` and its parts),
//! `Sec. 5`, `Section 5`, `Art. III`, `Article 3`, `Rule 113` and `Constitution` (plus the Tagalog
//! `Seksyon`, `Artikulo`, `Saligang Batas`). Pure: no IO, no allocation beyond the result.

use super::roman::from_roman;

/// A cited law.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LawRef {
    /// A Republic Act by number.
    RepublicAct(u32),
    /// The 1987 Constitution.
    Constitution,
    /// An Act of the Philippine Commission or Legislature (1900–1935), e.g. the Revised Penal Code
    /// (Act No. 3815).
    Act(u32),
    /// A Commonwealth Act.
    CommonwealthAct(u32),
    /// A Batas Pambansa.
    BatasPambansa(u32),
    /// A Presidential Decree, e.g. the Labor Code (P.D. No. 442).
    PresidentialDecree(u32),
    /// An Executive Order, e.g. the Family Code (E.O. No. 209).
    ExecutiveOrder(u32),
    /// A Letter of Instructions.
    LetterOfInstruction(u32),
    /// The Rules of Court (civil and criminal procedure, evidence).
    RulesOfCourt,
}

/// A cited provision inside a law.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProvisionRef {
    /// `Sec. n` / `Section n`.
    Section(u32),
    /// `Art. n` / `Article n`, with `n` given in Arabic or Roman numerals.
    Article(u32),
    /// `Rule n` (Rules of Court).
    Rule(u32),
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
            _ => None,
        })
    }

    /// The first cited article number, if any.
    #[must_use]
    pub fn article(&self) -> Option<u32> {
        self.provisions.iter().find_map(|p| match p {
            ProvisionRef::Article(n) => Some(*n),
            _ => None,
        })
    }

    /// The first cited rule number, if any.
    #[must_use]
    pub fn rule(&self) -> Option<u32> {
        self.provisions.iter().find_map(|p| match p {
            ProvisionRef::Rule(n) => Some(*n),
            _ => None,
        })
    }
}

/// Laws cited by name alone, longest phrase first where two share a start.
const NAMED: &[(&[&str], LawRef)] = &[
    (&["constitution"], LawRef::Constitution),
    (&["konstitusyon"], LawRef::Constitution),
    (&["saligang", "batas"], LawRef::Constitution),
    (&["revised", "penal", "code"], LawRef::Act(3815)),
    (&["rpc"], LawRef::Act(3815)),
    (&["family", "code"], LawRef::ExecutiveOrder(209)),
    (&["labor", "code"], LawRef::PresidentialDecree(442)),
    (&["labour", "code"], LawRef::PresidentialDecree(442)),
    (&["rules", "of", "court"], LawRef::RulesOfCourt),
    (&["rules", "of", "civil", "procedure"], LawRef::RulesOfCourt),
    (
        &["rules", "of", "criminal", "procedure"],
        LawRef::RulesOfCourt,
    ),
    (&["rules", "on", "evidence"], LawRef::RulesOfCourt),
    (&["rules", "of", "evidence"], LawRef::RulesOfCourt),
];

/// Reads one number token (Arabic, or Arabic or Roman).
type NumberReader = fn(&str) -> Option<u32>;

/// A law kind read from its number.
type Numbered = fn(u32) -> LawRef;

/// Heads followed by a number, with the largest number each accepts. `act` comes last, so
/// `Republic Act` and `Commonwealth Act` win.
const NUMBERED: &[(&[&str], Numbered, u32)] = &[
    (&["republic", "act"], LawRef::RepublicAct, MAX_NUMBER),
    (&["r", "a"], LawRef::RepublicAct, MAX_NUMBER),
    (&["ra"], LawRef::RepublicAct, MAX_NUMBER),
    (
        &["commonwealth", "act"],
        LawRef::CommonwealthAct,
        MAX_NUMBER,
    ),
    (&["c", "a"], LawRef::CommonwealthAct, MAX_NUMBER),
    (&["ca"], LawRef::CommonwealthAct, MAX_NUMBER),
    (&["batas", "pambansa"], LawRef::BatasPambansa, MAX_NUMBER),
    (&["b", "p"], LawRef::BatasPambansa, MAX_NUMBER),
    (&["bp"], LawRef::BatasPambansa, MAX_NUMBER),
    (
        &["presidential", "decree"],
        LawRef::PresidentialDecree,
        MAX_NUMBER,
    ),
    (&["p", "d"], LawRef::PresidentialDecree, MAX_NUMBER),
    (&["pd"], LawRef::PresidentialDecree, MAX_NUMBER),
    (&["executive", "order"], LawRef::ExecutiveOrder, MAX_NUMBER),
    (&["e", "o"], LawRef::ExecutiveOrder, MAX_NUMBER),
    (&["eo"], LawRef::ExecutiveOrder, MAX_NUMBER),
    (
        &["letter", "of", "instructions"],
        LawRef::LetterOfInstruction,
        MAX_NUMBER,
    ),
    (
        &["letter", "of", "instruction"],
        LawRef::LetterOfInstruction,
        MAX_NUMBER,
    ),
    (&["loi"], LawRef::LetterOfInstruction, MAX_NUMBER),
    (&["act"], LawRef::Act, ACT_MAX),
];

/// Heads written glued to their number, e.g. `RA9262`, `PD1866`.
const GLUED: &[(&str, Numbered)] = &[
    ("ra", LawRef::RepublicAct),
    ("pd", LawRef::PresidentialDecree),
    ("eo", LawRef::ExecutiveOrder),
    ("bp", LawRef::BatasPambansa),
    ("ca", LawRef::CommonwealthAct),
];

/// The last numbered Act (1935). A larger "Act N" is a named Republic Act written loosely
/// (`Cybercrime Prevention Act 10175`), so it is not read as an old Act.
const ACT_MAX: u32 = 4_275;

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

/// Whether `phrase` is spelled by the tokens at `i`.
fn starts_with(tokens: &[String], i: usize, phrase: &[&str]) -> bool {
    tokens
        .get(i..i + phrase.len())
        .is_some_and(|t| t.iter().zip(phrase).all(|(a, b)| a == b))
}

/// Tries every law form at `i`; returns the law and how many tokens it consumed.
fn law_at(tokens: &[String], i: usize) -> Option<(LawRef, usize)> {
    if let Some((phrase, law)) = NAMED.iter().find(|(p, _)| starts_with(tokens, i, p)) {
        return Some((*law, phrase.len()));
    }
    let t = tokens.get(i)?.as_str();
    let glued = GLUED
        .iter()
        .find_map(|(head, make)| t.strip_prefix(head).and_then(arabic).map(make));
    if let Some(law) = glued {
        return Some((law, 1));
    }
    NUMBERED.iter().find_map(|(phrase, make, max)| {
        if !starts_with(tokens, i, phrase) {
            return None;
        }
        let (n, used) = number_after(tokens, i + phrase.len(), arabic)?;
        (n <= *max).then(|| (make(n), phrase.len() + used))
    })
}

/// Tries every citation form at `i`; returns how many tokens it consumed.
fn match_at(tokens: &[String], i: usize, refs: &mut References) -> Option<usize> {
    if let Some((law, used)) = law_at(tokens, i) {
        push(&mut refs.laws, law);
        return Some(used);
    }
    let t = tokens.get(i)?.as_str();
    let (make, read): (fn(u32) -> ProvisionRef, NumberReader) = match t {
        "sec" | "secs" | "section" | "seksyon" => (ProvisionRef::Section, arabic),
        "art" | "arts" | "article" | "artikulo" => (ProvisionRef::Article, arabic_or_roman),
        "rule" | "rules" => (ProvisionRef::Rule, arabic),
        _ => return None,
    };
    let (n, used) = number_after(tokens, i + 1, read)?;
    push(&mut refs.provisions, make(n));
    Some(1 + used)
}

/// Reads an optional `No.` / `Number` / `Blg.` marker, then a number; returns it and tokens used.
fn number_after(tokens: &[String], i: usize, read: NumberReader) -> Option<(u32, usize)> {
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
