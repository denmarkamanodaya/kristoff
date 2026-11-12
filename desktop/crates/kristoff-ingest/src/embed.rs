//! Resumable passage embedding, shared by the statute chunks and the decisions.

use std::time::Instant;

use anyhow::{Result, anyhow};
use kristoff_embed::CandleEmbedder;
use kristoff_law::quant::quantize;
use rusqlite::{Connection, params};

/// Where the pending texts come from and where their vectors go.
pub struct Target<'a> {
    /// What the progress lines call the items.
    pub what: &'a str,
    /// `SELECT id, text` of the items that have no vector yet.
    pub pending_sql: &'a str,
    /// `INSERT OR IGNORE … (id, scale, v) VALUES (?1, ?2, ?3)`: two runs may embed the same db
    /// from opposite ends (`longest_first`) and meet in the middle.
    pub insert_sql: &'a str,
    /// Embed the longest texts first (a second run beside a shortest-first one).
    pub longest_first: bool,
}

/// Embeds every pending item with the `"passage: "` prefix, batch by batch, committing each
/// batch so a stopped run resumes where it ended. Returns how many it embedded.
pub fn run(
    conn: &mut Connection,
    embedder: &CandleEmbedder,
    batch: usize,
    target: &Target<'_>,
) -> Result<usize> {
    let mut pending: Vec<(i64, String)> = conn
        .prepare(target.pending_sql)?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    // Similar lengths per batch keep padding (and wasted attention) low.
    pending.sort_by_key(|(_, text)| text.len());
    if target.longest_first {
        pending.reverse();
    }
    let total = pending.len();
    eprintln!(
        "embed: {total} {} pending on {}, batch {batch}",
        target.what,
        embedder.device_name()
    );
    let t = Instant::now();
    let mut done = 0usize;
    for group in pending.chunks(batch) {
        let texts: Vec<&str> = group.iter().map(|(_, s)| s.as_str()).collect();
        let vectors = embedder
            .embed_passages(&texts)
            .map_err(|e| anyhow!(e.to_string()))?;
        let tx = conn.transaction()?;
        {
            let mut ins = tx.prepare_cached(target.insert_sql)?;
            for ((id, _), v) in group.iter().zip(&vectors) {
                let (bytes, scale) = quantize(v);
                ins.execute(params![id, f64::from(scale), bytes])?;
            }
        }
        tx.commit()?;
        done += group.len();
        if done % (batch * 250) < batch || done == total {
            let rate =
                f64::from(u32::try_from(done).unwrap_or(u32::MAX)) / t.elapsed().as_secs_f64();
            eprintln!("embed: {done}/{total} {} ({rate:.1}/s)", target.what);
        }
    }
    Ok(total)
}
