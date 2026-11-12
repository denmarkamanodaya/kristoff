//! `--omni-report`: splitter accuracy on the codes, checked against their known article counts.

use std::path::Path;

use anyhow::Result;
use kristoff_law::schema::kind;

use super::split::split;
use super::{read, select, text};

/// Articles the codes are known to have (numbers 1..=n): RPC 367 (Act 3815, 1930) and Family
/// Code 257 (EO 209, 1987). The 1974 Labor Code count is read from its text, not assumed.
const KNOWN: [(&str, u32); 2] = [
    ("Revised Penal Code", 367),
    ("Family Code of the Philippines", 257),
];

/// One line per code and rules document: provisions found, numbering range, missing numbers.
pub fn lines(parquet: &Path) -> Result<Vec<String>> {
    let rows = read::rows(parquet, |info, label| {
        select::statute(info, label)
            .is_some_and(|s| s.kind == kind::RULES || s.identifier.ends_with(')'))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let Some(statute) = select::statute(&row.info, &row.label) else {
            continue;
        };
        let provisions = split(&text::clean_omni(&row.text));
        let numbers: Vec<u32> = provisions
            .iter()
            .filter_map(|p| p.label.rsplit(' ').next()?.split('-').next()?.parse().ok())
            .collect();
        let max = numbers.iter().copied().max().unwrap_or(0);
        let expected = KNOWN
            .iter()
            .find(|(name, _)| statute.identifier.contains(name))
            .map(|(_, n)| *n);
        let missing: Vec<u32> = (1..=expected.unwrap_or(max))
            .filter(|k| !numbers.contains(k))
            .collect();
        let extra: Vec<&str> = provisions
            .iter()
            .map(|p| p.label.as_str())
            .filter(|l| l.contains('-'))
            .collect();
        out.push(format!(
            "{}: {} provisions, numbers up to {max}, expected {expected:?}, missing {} {:?}, suffixed {:?}",
            statute.identifier,
            provisions.len(),
            missing.len(),
            &missing[..missing.len().min(20)],
            &extra[..extra.len().min(12)]
        ));
    }
    Ok(out)
}
