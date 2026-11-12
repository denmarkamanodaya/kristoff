//! Chunking (Phase 3 §1.4, revised 2026-11-12): one chunk per article; an article over the
//! token budget is split on paragraph, line and sentence boundaries (word windows only for a
//! single over-long sentence). Each part after the first repeats the previous part's last
//! sentence, recorded as `overlap` so `kristoff_core::domain::law::provision::stitch` restores the exact text.

use kristoff_core::domain::law::provision::Part;

/// Token budget per chunk, prefix included, counted with the e5 tokenizer. Under e5-small's
/// 512-token window, so every part is embedded whole.
pub const MAX_CHUNK_TOKENS: usize = 480;

/// Smallest text budget left after the prefix, so a very long title cannot starve the text.
const MIN_TEXT_BUDGET: usize = 64;

/// A sentence-sized piece of text and the separator that came before it in the source.
struct Unit {
    sep: &'static str,
    text: String,
}

type Counter<'a, E> = &'a dyn Fn(&str) -> Result<usize, E>;

/// Splits `text` into parts whose `count(prefix + part)` stays within [`MAX_CHUNK_TOKENS`].
///
/// Returns one part (overlap 0) when the whole article already fits.
pub fn split<E>(prefix: &str, text: &str, count: Counter<'_, E>) -> Result<Vec<Part>, E> {
    if count(&format!("{prefix}{text}"))? <= MAX_CHUNK_TOKENS {
        return Ok(vec![Part {
            text: text.to_owned(),
            overlap: 0,
        }]);
    }
    let budget = MAX_CHUNK_TOKENS
        .saturating_sub(count(prefix)?)
        .max(MIN_TEXT_BUDGET);
    let mut units = Vec::new();
    for unit in units_of(text) {
        explode(unit, budget, count, &mut units)?;
    }
    pack(&units, budget, count)
}

/// Splits text into paragraphs, lines and sentences, remembering each separator.
fn units_of(text: &str) -> Vec<Unit> {
    let mut out = Vec::new();
    for (p, para) in text.split("\n\n").enumerate() {
        for (l, line) in para.split('\n').enumerate() {
            for (s, sentence) in sentences(line).into_iter().enumerate() {
                let sep = match (p, l, s) {
                    (0, 0, 0) => "",
                    (_, 0, 0) => "\n\n",
                    (_, _, 0) => "\n",
                    _ => "",
                };
                out.push(Unit {
                    sep,
                    text: sentence.to_owned(),
                });
            }
        }
    }
    out
}

/// Sentences of one line; each keeps its trailing space so concatenation is lossless.
fn sentences(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, _) in line.match_indices(['.', '?', '!', ';']) {
        let end = i + 1;
        if line[end..].starts_with(' ') {
            out.push(&line[start..=end]);
            start = end + 1;
        }
    }
    if start < line.len() || out.is_empty() {
        out.push(&line[start..]);
    }
    out
}

/// Breaks a unit that alone exceeds `budget` into word windows that fit.
fn explode<E>(
    unit: Unit,
    budget: usize,
    count: Counter<'_, E>,
    out: &mut Vec<Unit>,
) -> Result<(), E> {
    if count(&unit.text)? <= budget {
        out.push(unit);
        return Ok(());
    }
    let words: Vec<&str> = unit.text.split_inclusive(' ').collect();
    let mut window = String::new();
    let mut sep = unit.sep;
    for word in words {
        let candidate = format!("{window}{word}");
        if !window.is_empty() && count(&candidate)? > budget {
            out.push(Unit {
                sep,
                text: std::mem::take(&mut window),
            });
            sep = "";
            window.push_str(word);
        } else {
            window = candidate;
        }
    }
    if !window.is_empty() {
        out.push(Unit { sep, text: window });
    }
    Ok(())
}

/// Greedily packs units into parts; each new part starts with the previous part's last unit.
fn pack<E>(units: &[Unit], budget: usize, count: Counter<'_, E>) -> Result<Vec<Part>, E> {
    let mut parts: Vec<Part> = Vec::new();
    let mut current = String::new();
    let mut overlap = 0;
    let mut has_new = false;
    let mut last: Option<&Unit> = None;
    for unit in units {
        let candidate = if current.is_empty() {
            unit.text.clone()
        } else {
            format!("{current}{}{}", unit.sep, unit.text)
        };
        if !has_new || count(&candidate)? <= budget {
            current = candidate;
            has_new = true;
        } else {
            parts.push(Part {
                text: std::mem::take(&mut current),
                overlap,
            });
            let lead = last.map(|l| l.text.as_str()).unwrap_or_default();
            let with_lead = format!("{lead}{}{}", unit.sep, unit.text);
            if !lead.is_empty() && count(&with_lead)? <= budget {
                current = with_lead;
                overlap = lead.len();
            } else {
                current = format!("{}{}", unit.sep, unit.text);
                overlap = 0;
            }
        }
        last = Some(unit);
    }
    if has_new {
        parts.push(Part {
            text: current,
            overlap,
        });
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use kristoff_core::domain::law::provision::stitch;
    use proptest::prelude::*;

    use super::*;

    fn count_words(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[allow(clippy::unnecessary_wraps)] // Must match the counter signature `split` takes.
    fn words(s: &str) -> Result<usize, ()> {
        Ok(count_words(s))
    }

    fn joined(parts: &[Part]) -> String {
        stitch(parts.iter().map(|p| (p.text.as_str(), p.overlap)))
    }

    #[test]
    fn keeps_a_short_article_as_one_chunk() {
        let parts = split("P\n", "one two.", &words).unwrap();
        assert_eq!(
            parts,
            vec![Part {
                text: "one two.".into(),
                overlap: 0
            }]
        );
    }

    #[test]
    fn splits_a_long_article_within_the_budget_with_one_sentence_of_overlap() {
        let sentence = format!("{}.", vec!["w"; 100].join(" "));
        let text = [sentence.as_str(); 12].join(" ");

        let parts = split("P\n", &text, &words).unwrap();

        assert!(parts.len() > 2);
        assert!(
            parts
                .iter()
                .all(|p| count_words(&format!("P\n{}", p.text)) <= MAX_CHUNK_TOKENS)
        );
        assert!(parts.iter().skip(1).all(|p| p.overlap > 0));
        assert_eq!(joined(&parts), text);
    }

    proptest! {
        #[test]
        fn stitching_the_parts_restores_the_article(
            sentences in prop::collection::vec("[a-z]{1,8}( [a-z]{1,8}){0,150}[.;]", 1..30),
            seps in prop::collection::vec(prop::sample::select(vec![" ", "\n", "\n\n"]), 30),
        ) {
            let mut text = String::new();
            for (i, s) in sentences.iter().enumerate() {
                if i > 0 { text.push_str(seps[i]); }
                text.push_str(s);
            }

            let parts = split("P\n", &text, &words).unwrap();

            prop_assert_eq!(joined(&parts), text.clone());
            for p in &parts {
                let tokens = count_words(&format!("P\n{}", p.text));
                prop_assert!(tokens <= MAX_CHUNK_TOKENS, "part has {} tokens", tokens);
            }
        }
    }
}
