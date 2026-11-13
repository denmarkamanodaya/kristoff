//! `--repair-case-names`: re-applies the text repair to the decision metadata of an existing
//! `phcases.db` (titles, case names, ponentes) without touching passages, FTS or vectors.
//! Idempotent: repaired text is left as it is.

use std::path::Path;

use anyhow::Result;
use rusqlite::{Connection, params};

use crate::omni::text::fix_mojibake;

/// Repairs every `cases` row whose title, case name or ponente changes; returns rows updated.
///
/// # Errors
/// When the database cannot be opened or written.
pub fn run(phcases_db: &Path) -> Result<usize> {
    let mut conn = Connection::open(phcases_db)?;
    let rows: Vec<(i64, String, String, Option<String>)> = conn
        .prepare("SELECT id, title, case_name, ponente FROM cases")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    let tx = conn.transaction()?;
    let mut changed = 0;
    {
        let mut update =
            tx.prepare("UPDATE cases SET title = ?2, case_name = ?3, ponente = ?4 WHERE id = ?1")?;
        for (id, title, case_name, ponente) in rows {
            let fixed = (
                fix_mojibake(&title).0,
                fix_mojibake(&case_name).0,
                ponente.as_deref().map(|p| fix_mojibake(p).0),
            );
            if (fixed.0.as_str(), fixed.1.as_str(), fixed.2.as_deref())
                != (title.as_str(), case_name.as_str(), ponente.as_deref())
            {
                update.execute(params![id, fixed.0, fixed.1, fixed.2])?;
                changed += 1;
            }
        }
    }
    tx.commit()?;
    Ok(changed)
}
