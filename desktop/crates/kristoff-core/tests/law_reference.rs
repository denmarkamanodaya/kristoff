//! Property tests for the reference parser and Roman numerals (Phase 3 exit criterion).

use kristoff_core::domain::law::reference::{LawRef, ProvisionRef, parse_references};
use kristoff_core::domain::law::roman::{from_roman, to_roman};
use proptest::prelude::*;

const RA_FORMS: [&str; 7] = [
    "RA {}",
    "R.A. {}",
    "R.A. No. {}",
    "RA No. {}",
    "Republic Act {}",
    "Republic Act No. {}",
    "RA{}",
];
const SEC_FORMS: [&str; 5] = [
    "Sec. {}",
    "Section {}",
    "SEC {}",
    "section no. {}",
    "Seksyon {}",
];
const ART_FORMS: [&str; 4] = ["Art. {}", "Article {}", "ARTICLE {}", "Artikulo {}"];

fn fill(form: &str, n: impl std::fmt::Display) -> String {
    form.replace("{}", &n.to_string())
}

proptest! {
    #[test]
    fn roman_numerals_round_trip(n in 1u32..=3999) {
        let r = to_roman(n).unwrap();
        prop_assert_eq!(from_roman(&r), Some(n));
        prop_assert_eq!(from_roman(&r.to_uppercase()), Some(n));
    }

    #[test]
    fn every_republic_act_form_is_read(n in 1u32..=99_999, form in 0usize..RA_FORMS.len(), pad in "[ a-z]{0,12}") {
        let text = format!("{pad} {} {pad}", fill(RA_FORMS[form], n));
        prop_assert!(parse_references(&text).laws.contains(&LawRef::RepublicAct(n)), "{}", text);
    }

    #[test]
    fn every_section_form_is_read(n in 1u32..=99_999, form in 0usize..SEC_FORMS.len()) {
        let refs = parse_references(&fill(SEC_FORMS[form], n));
        prop_assert_eq!(refs.provisions, vec![ProvisionRef::Section(n)]);
    }

    #[test]
    fn articles_read_in_arabic_and_roman(n in 1u32..=3999, form in 0usize..ART_FORMS.len(), roman in any::<bool>()) {
        let num = if roman { to_roman(n).unwrap().to_uppercase() } else { n.to_string() };
        let refs = parse_references(&fill(ART_FORMS[form], num));
        prop_assert_eq!(refs.article(), Some(n));
    }

    #[test]
    fn a_full_citation_yields_law_and_provision(n in 1u32..=99_999, s in 1u32..=500) {
        let refs = parse_references(&format!("R.A. No. {n}, Sec. {s}"));
        prop_assert_eq!(&refs.laws, &vec![LawRef::RepublicAct(n)]);
        prop_assert_eq!(refs.section(), Some(s));
    }

    #[test]
    fn arbitrary_text_never_panics_and_finds_nothing_without_markers(text in "[a-z ]{0,80}") {
        let refs = parse_references(&text);
        prop_assert!(refs.provisions.is_empty());
    }

    #[test]
    fn any_unicode_input_is_safe(text in "\\PC{0,120}") {
        let _ = parse_references(&text);
    }
}

#[test]
fn reads_the_constitution_citation_with_article_and_section() {
    let refs = parse_references("Article III Sec. 2 Constitution");

    assert_eq!(refs.laws, vec![LawRef::Constitution]);
    assert_eq!(refs.article(), Some(3));
    assert_eq!(refs.section(), Some(2));
}

#[test]
fn rejects_non_canonical_roman_numerals() {
    assert_eq!(from_roman("iiii"), None);
    assert_eq!(from_roman("ic"), None);
    assert_eq!(from_roman(""), None);
}

#[test]
fn ignores_digit_runs_longer_than_five() {
    assert_eq!(parse_references("RA 1234567").laws, Vec::<LawRef>::new());
}
