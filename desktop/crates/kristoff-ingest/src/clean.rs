//! Cleans dataset text (Phase 3 §1.3): stray leading quotes, scraper footers, whitespace, NFC,
//! truncation marks. Pure functions; counts are kept in [`CleanStats`].

use tokenizers::NormalizedString;
use tokenizers::normalizers::unicode::NFC;
use tokenizers::tokenizer::Normalizer;

/// Footer the scraper appended to Supreme Court E-Library pages.
const FOOTER_MARKERS: [&str; 2] = ["Source: Supreme Court E-Library", "Back to top"];

/// How many rows each fix touched.
#[derive(Debug, Default)]
pub struct CleanStats {
    pub leading_junk_trimmed: usize,
    pub footer_removed: usize,
    pub whitespace_collapsed: usize,
    pub nfc_changed: usize,
    pub truncated: usize,
    pub empty_dropped: usize,
    pub duplicate_dropped: usize,
    pub heading_dropped: usize,
}

/// NFC-normalises `text`.
pub fn nfc(text: &str) -> String {
    let mut n = NormalizedString::from(text);
    match NFC.normalize(&mut n) {
        Ok(()) => n.get().to_owned(),
        // NFC normalisation of valid UTF-8 cannot fail; keep the input if it ever does.
        Err(_) => text.to_owned(),
    }
}

/// Trims stray leading `">`, quotes and whitespace left by the HTML scraper.
pub fn trim_leading_junk(text: &str) -> &str {
    text.trim_start_matches(|c: char| c == '"' || c == '>' || c == '\'' || c.is_whitespace())
}

/// Cuts the scraper footer and everything after it.
pub fn strip_footer(text: &str) -> &str {
    FOOTER_MARKERS
        .iter()
        .filter_map(|m| text.find(m))
        .min()
        .map_or(text, |at| &text[..at])
}

/// Collapses runs of spaces inside lines and runs of blank lines to one blank line.
///
/// Paragraph breaks survive as `"\n\n"` so the chunker can split on them.
pub fn collapse_whitespace(text: &str) -> String {
    let mut paragraphs: Vec<String> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join("\n"));
                current.clear();
            }
        } else {
            current.push(words.join(" "));
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join("\n"));
    }
    paragraphs.join("\n\n")
}

/// True when `text` ends mid-sentence (a letter, digit or comma as its last character).
pub fn looks_truncated(text: &str) -> bool {
    text.trim_end()
        .chars()
        .last()
        .is_some_and(|c| c.is_alphanumeric() || c == ',')
}

/// Runs every text fix in order, counting each one that changed something.
pub fn clean_text(raw: &str, stats: &mut CleanStats) -> String {
    let trimmed = trim_leading_junk(raw);
    if trimmed.len() != raw.len() {
        stats.leading_junk_trimmed += 1;
    }
    let cut = strip_footer(trimmed);
    if cut.len() != trimmed.len() {
        stats.footer_removed += 1;
    }
    let collapsed = collapse_whitespace(cut);
    if collapsed != cut {
        stats.whitespace_collapsed += 1;
    }
    let normalised = nfc(&collapsed);
    if normalised != collapsed {
        stats.nfc_changed += 1;
    }
    normalised
}

/// Cleans a one-line field (title, label): junk trim, whitespace collapse to single spaces, NFC.
pub fn clean_line(raw: &str) -> String {
    let words: Vec<&str> = trim_leading_junk(raw).split_whitespace().collect();
    nfc(words.join(" ").trim_end_matches(['.', ' ']))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_the_scraper_quote_and_angle_bracket() {
        assert_eq!(trim_leading_junk("\">AN ACT"), "AN ACT");
    }

    #[test]
    fn strips_the_e_library_footer() {
        let raw = "SEC. 3. Effective.Approved, April 16, 1947.\n\nSource: Supreme Court E-Library\n\nBack to top";
        assert_eq!(
            strip_footer(raw).trim_end(),
            "SEC. 3. Effective.Approved, April 16, 1947."
        );
    }

    #[test]
    fn keeps_paragraph_breaks_and_collapses_inner_spaces() {
        assert_eq!(collapse_whitespace("a   b\n\n\n\n  c\td  \n"), "a b\n\nc d");
    }

    #[test]
    fn marks_text_ending_mid_sentence_as_truncated() {
        assert!(looks_truncated("The crime is punished under"));
        assert!(!looks_truncated("This Act shall take effect."));
        assert!(!looks_truncated(
            "committed through any of the following acts:"
        ));
    }
}
