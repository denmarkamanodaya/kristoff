//! `OmniCorpus` text repair (Phase 7): mojibake from UTF-8 read as Latin-1, the `E-Library` page
//! header and footer, and whitespace. Pure functions; `docs/desktop/phases/phase-7-omnicorpus.md`.

use crate::clean;

/// Longest prefix (in chars) searched for the `[ ACT NO. 3815, December 08, 1930 ]` page header.
const HEADER_WINDOW: usize = 600;

/// Repairs UTF-8 sequences that were decoded as Latin-1 (`â\u{80}\u{94}` becomes `—`) and drops
/// leftover C1 control characters. Returns the text and whether anything changed.
#[must_use]
pub fn fix_mojibake(text: &str) -> (String, bool) {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut changed = false;
    let mut i = 0;
    while i < chars.len() {
        if let Some((decoded, used)) = decode_at(&chars, i) {
            if !('\u{80}'..='\u{9f}').contains(&decoded) {
                out.push(decoded);
            }
            changed = true;
            i += used;
            continue;
        }
        let c = chars[i];
        if ('\u{80}'..='\u{9f}').contains(&c) {
            changed = true;
            i += 1;
        } else if let Some((repair, used)) = lost_continuation(&chars, i) {
            out.push_str(repair);
            changed = true;
            i += used;
        } else {
            out.push(c);
            i += 1;
        }
    }
    (out, changed)
}

/// A lead byte whose continuation bytes the source already lost (`â ` for a curly quote,
/// apostrophe or dash; `Ã ` for ñ; `Â ` for a no-break space). The punctuation cannot be
/// recovered, so: `â` before a contraction (`Minguezâ s` → `Minguez's`) becomes an apostrophe,
/// `Ã` between letters becomes ñ / Ñ, and any other stray lead byte is dropped.
/// Returns the replacement and the chars it consumed.
fn lost_continuation(chars: &[char], i: usize) -> Option<(&'static str, usize)> {
    let c = chars[i];
    if !matches!(c, '\u{c2}' | '\u{c3}' | '\u{e2}') {
        return None;
    }
    let prev_letter = i > 0 && chars[i - 1].is_alphabetic();
    let space_then = |k: usize| chars.get(i + 1) == Some(&' ') && chars.get(i + 1 + k).is_some();
    let next = chars.get(i + 2).copied();
    if c == '\u{e2}' && prev_letter && space_then(1) && starts_contraction(&chars[i + 2..]) {
        return Some(("'", 2));
    }
    if c == '\u{c3}' && prev_letter && space_then(1) && next.is_some_and(char::is_alphabetic) {
        let upper = next.is_some_and(char::is_uppercase);
        return Some((if upper { "\u{d1}" } else { "\u{f1}" }, 2));
    }
    Some(("", 1))
}

/// `s`, `t`, `d`, `m`, `ll`, `re`, `ve` followed by a non-letter.
fn starts_contraction(rest: &[char]) -> bool {
    let word: String = rest.iter().take_while(|c| c.is_alphabetic()).collect();
    matches!(word.as_str(), "s" | "t" | "d" | "m" | "ll" | "re" | "ve")
}

/// Decodes the UTF-8 sequence spelled by Latin-1 chars at `i`, if there is a valid one.
fn decode_at(chars: &[char], i: usize) -> Option<(char, usize)> {
    let lead = u32::from(*chars.get(i)?);
    let len = match lead {
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => return None,
    };
    let mut bytes = Vec::with_capacity(len);
    for c in chars.get(i..i + len)? {
        bytes.push(u8::try_from(u32::from(*c)).ok()?);
    }
    if !bytes[1..].iter().all(|b| (0x80..=0xBF).contains(b)) {
        return None;
    }
    let s = std::str::from_utf8(&bytes).ok()?;
    s.chars().next().map(|c| (c, len))
}

/// Drops everything up to the first `[ … ]` page header when it sits near the start.
#[must_use]
pub fn strip_header(text: &str) -> &str {
    let window: usize = text
        .char_indices()
        .nth(HEADER_WINDOW)
        .map_or(text.len(), |(i, _)| i);
    let Some(open) = text[..window].find("[ ") else {
        return text;
    };
    let Some(close) = text[open..window].find(']') else {
        return text;
    };
    text[open + close + 1..].trim_start()
}

/// The full repair: mojibake, footer, header, whitespace and NFC.
#[must_use]
pub fn clean_omni(raw: &str) -> String {
    let (fixed, _) = fix_mojibake(raw);
    let body = strip_header(clean::strip_footer(&fixed));
    clean::nfc(&body.split_whitespace().collect::<Vec<_>>().join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_utf8_read_as_latin1_and_drops_stray_controls() {
        let raw = "Fine\u{e2}\u{80}\u{94} When afflictive. MARI\u{c3}\u{b1}O \u{c2}\u{80}x";

        let (fixed, changed) = fix_mojibake(raw);

        assert!(changed);
        assert_eq!(
            fixed,
            "Fine\u{2014} When afflictive. MARI\u{f1}O \u{80}x".replace('\u{80}', "")
        );
    }

    #[test]
    fn repairs_lead_bytes_whose_continuation_was_lost() {
        let raw = "made a \u{e2} missed call\u{e2} to SA Minguez\u{e2} s phone. MARI\u{c3} O and Pe\u{c3} a.\u{c2} End";

        let (fixed, changed) = fix_mojibake(raw);

        assert!(changed);
        assert_eq!(
            fixed,
            "made a  missed call to SA Minguez's phone. MARI\u{d1}O and Pe\u{f1}a. End"
        );
    }

    #[test]
    fn leaves_a_genuine_latin1_letter_alone() {
        assert_eq!(
            fix_mojibake("Ma\u{f1}o, caf\u{e9}"),
            ("Ma\u{f1}o, caf\u{e9}".to_owned(), false)
        );
    }

    #[test]
    fn strips_the_page_header_and_footer() {
        let raw = "H. No. 1 / 55 OG No. 20 [ REPUBLIC ACT NO. 2137, April 07, 1959 ]AN ACT X. \
                   Source: Supreme Court E-LibraryThis page was dynamically generated";

        assert_eq!(clean_omni(raw), "AN ACT X.");
    }
}
