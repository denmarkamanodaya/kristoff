//! Writes `OmniCorpus` statutes into `phlaws.db` (Phase 7): one law per row, one provision per
//! article or section, each law labelled with its edition (`As enacted`, `As promulgated`,
//! `As issued`). The current dataset wins on duplicate RAs.

use std::collections::HashSet;

use anyhow::Result;
use kristoff_law::schema::{SOURCE_OMNI, kind};
use rusqlite::Connection;

use super::read::Row;
use super::select::{self, Statute};
use super::split::split;
use super::text::{clean_omni, fix_mojibake};
use crate::build::{self, Article, ChunkTarget, Law, Written};
use crate::clean;

/// What the `OmniCorpus` statute stage did.
#[derive(Debug, Default)]
pub struct OmniWritten {
    /// Laws, chunks and split provisions written.
    pub written: Written,
    /// RAs skipped because the current dataset already has them.
    pub duplicate_ras: usize,
    /// Rows whose text had mojibake repaired.
    pub mojibake_fixed: usize,
}

/// The RA numbers already in `phlaws.db` (from the current dataset).
///
/// # Errors
/// When the query fails.
pub fn existing_ras(conn: &Connection) -> Result<HashSet<u32>> {
    let mut stmt =
        conn.prepare("SELECT kind_number FROM laws WHERE kind = ?1 AND kind_number IS NOT NULL")?;
    let rows = stmt.query_map([kind::RA], |r| r.get::<_, u32>(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// `E-Library .../showdocs/28/20426` becomes `omni-28-20426`.
fn source_id(url: &str) -> String {
    let tail: Vec<&str> = url.trim_end_matches('/').rsplit('/').take(2).collect();
    format!(
        "omni-{}",
        tail.iter().rev().copied().collect::<Vec<_>>().join("-")
    )
}

fn law_of(row: &Row, statute: &Statute) -> Law {
    let (title, _) = fix_mojibake(row.info.title.as_deref().unwrap_or(&statute.identifier));
    Law {
        source_id: source_id(&row.url),
        identifier: statute.identifier.clone(),
        number: (statute.kind == kind::RA)
            .then_some(statute.number)
            .flatten(),
        is_constitution: false,
        title: clean::clean_line(&title),
        year: row.info.any_date().and_then(|d| d.get(..4)?.parse().ok()),
        status: "current".to_owned(),
        source_url: row.url.clone(),
        dataset_id: SOURCE_OMNI,
        kind: statute.kind,
        kind_number: statute.number,
        edition: Some(statute.edition),
    }
}

fn articles_of(row: &Row) -> Vec<Article> {
    split(&clean_omni(&row.text))
        .into_iter()
        .map(|p| Article {
            label: p.label,
            text: p.text,
            truncated: false,
            rule: p.rule,
        })
        .collect()
}

/// Writes every selected statute row not already present, in one transaction.
///
/// # Errors
/// When a write or the token counter fails.
pub fn write(
    conn: &mut Connection,
    rows: &[Row],
    count: &dyn Fn(&str) -> Result<usize>,
) -> Result<OmniWritten> {
    let have = existing_ras(conn)?;
    let written_before: HashSet<String> = conn
        .prepare("SELECT source_id FROM laws")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut out = OmniWritten::default();
    let tx = conn.transaction()?;
    {
        let mut ins_law = tx.prepare(build::INSERT_LAW)?;
        let mut ins_chunk = tx.prepare(build::INSERT_CHUNK)?;
        for row in rows {
            let Some(statute) = select::statute(&row.info, &row.label) else {
                continue;
            };
            if statute.kind == kind::RA && statute.number.is_some_and(|n| have.contains(&n)) {
                out.duplicate_ras += 1;
                continue;
            }
            out.mojibake_fixed += usize::from(fix_mojibake(&row.text).1);
            let law = law_of(row, &statute);
            if written_before.contains(&law.source_id) {
                continue; // written by an earlier run of this stage
            }
            build::insert_law(&mut ins_law, &law)?;
            out.written.laws += 1;
            let target = ChunkTarget {
                law_id: tx.last_insert_rowid(),
                law: &law,
                count,
            };
            build::insert_chunks(&mut ins_chunk, &target, &articles_of(row), &mut out.written)?;
        }
    }
    tx.commit()?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_a_stable_source_id_from_the_e_library_url() {
        assert_eq!(
            source_id("https://elibrary.judiciary.gov.ph/thebookshelf/showdocs/28/20426"),
            "omni-28-20426"
        );
    }
}
