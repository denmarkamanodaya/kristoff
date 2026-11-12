//! `get_article`: a whole provision, stitched from its parts, with its neighbours.

use kristoff_core::domain::law::provision::stitch;
use kristoff_core::domain::law::reference::parse_references;
use kristoff_core::domain::law::{ArticleView, LawHit, LawRef};
use rusqlite::{Connection, params};

use super::{HIT_SELECT, LawIndex, hit_from_row};
use crate::error::LawIndexError;

impl LawIndex {
    /// The provision `article` (e.g. `Sec. 5`) of `law`, stitched whole, with the provisions
    /// before and after it; `None` when no chunk matches.
    ///
    /// # Errors
    /// [`LawIndexError::Db`] when a query fails.
    pub fn article(
        &self,
        law: LawRef,
        article: &str,
    ) -> Result<Option<ArticleView>, LawIndexError> {
        let refs = parse_references(article);
        let law_ids = self.law_ids_for(law)?;
        let Some(&first) = self.provision_hits(&law_ids, &refs)?.first() else {
            return Ok(None);
        };
        let conn = self.conn()?;
        let (law_id, prov): (i64, i64) = conn.query_row(
            "SELECT law_id, prov FROM chunks WHERE id = ?1",
            [first],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let Some(hit) = load_provision(&conn, law_id, prov)? else {
            return Ok(None);
        };
        Ok(Some(ArticleView {
            previous: load_provision(&conn, law_id, prov - 1)?,
            next: load_provision(&conn, law_id, prov + 1)?,
            hit,
        }))
    }
}

/// Every part of provision `prov` in `law_id`, stitched into one hit; `None` when absent.
fn load_provision(
    conn: &Connection,
    law_id: i64,
    prov: i64,
) -> Result<Option<LawHit>, LawIndexError> {
    let sql = format!("{HIT_SELECT} WHERE c.law_id = ?1 AND c.prov = ?2 ORDER BY c.part");
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(params![law_id, prov], |r| {
        Ok((hit_from_row(r)?, r.get::<_, i64>(14)?))
    })?;
    let mut first: Option<LawHit> = None;
    let mut pieces: Vec<(String, usize)> = Vec::new();
    for row in rows {
        let (hit, overlap) = row?;
        pieces.push((hit.text.clone(), usize::try_from(overlap).unwrap_or(0)));
        first.get_or_insert(hit);
    }
    Ok(first.map(|mut hit| {
        hit.text = stitch(pieces.iter().map(|(t, o)| (t.as_str(), *o)));
        hit
    }))
}
