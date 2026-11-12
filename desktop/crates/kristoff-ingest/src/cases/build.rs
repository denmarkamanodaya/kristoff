//! Writes `phcases.db` (Phase 7, D-20): one row per decision, its text chunked fully into
//! 480-token passages with the statute chunker (overlap recorded, so passages stitch back), in
//! parallel batches. Vectors are added afterwards by the shared embed stage.

use std::path::Path;

use anyhow::{Result, anyhow};
use kristoff_core::domain::law::provision::Part;
use kristoff_law::case_schema::passage_prefix;
use kristoff_law::docket::{gr_display, gr_key};
use rusqlite::{Connection, params};
use tokenizers::Tokenizer;

use super::meta::{docket_and_division, ponente, short_title};
use crate::chunk;
use crate::omni::read::Row;
use crate::omni::text::{clean_omni, fix_mojibake};

/// Decisions chunked per parallel batch (bounds memory).
const BATCH: usize = 1_000;

/// One decision ready to insert.
struct Record {
    case_id: String,
    docket: Option<String>,
    gr_key: Option<String>,
    title: String,
    case_name: String,
    date: String,
    year: i64,
    ponente: Option<String>,
    division: Option<String>,
    url: String,
    prefix: String,
    parts: Vec<Part>,
}

/// Counts written.
#[derive(Debug, Default)]
pub struct CasesWritten {
    /// Decisions.
    pub cases: usize,
    /// Passages.
    pub passages: usize,
    /// Decisions left out (no usable date or empty text).
    pub skipped: usize,
}

fn record(row: &Row, tok: &Tokenizer) -> Result<Option<Record>> {
    let date = row.info.date.clone().unwrap_or_default();
    let Some(year) = date.get(..4).and_then(|y| y.parse::<i64>().ok()) else {
        return Ok(None);
    };
    let (docket, division) = docket_and_division(&row.text);
    let text = clean_omni(&row.text);
    if text.is_empty() {
        return Ok(None);
    }
    let (case_name, _) = fix_mojibake(row.info.case_name.as_deref().unwrap_or(""));
    let title = short_title(&case_name);
    let key = docket
        .as_deref()
        .and_then(|d| gr_key(d, false))
        .or_else(|| row.info.citation.as_deref().and_then(|c| gr_key(c, false)));
    let prefix = passage_prefix(&title, docket.as_deref(), year);
    let parts = passages(&prefix, &text, tok)?;
    Ok(Some(Record {
        case_id: case_id(&row.url),
        docket,
        gr_key: key,
        title,
        case_name,
        date,
        year,
        ponente: ponente(&text),
        division,
        url: row.url.clone(),
        prefix,
        parts,
    }))
}

/// `…/showdocs/1/65677` becomes `sc-65677`.
fn case_id(url: &str) -> String {
    format!(
        "sc-{}",
        url.trim_end_matches('/').rsplit('/').next().unwrap_or("")
    )
}

/// The decision text in 480-token passages, counted with `tok`.
fn passages(prefix: &str, text: &str, tok: &Tokenizer) -> Result<Vec<Part>> {
    let count = |s: &str| {
        tok.encode(s, true)
            .map(|e| e.len())
            .map_err(|e| anyhow!(e.to_string()))
    };
    chunk::split(prefix, text, &count)
}

fn chunk_batch(rows: &[Row], tokenizer: &Tokenizer, threads: usize) -> Result<Vec<Option<Record>>> {
    let per = rows.len().div_ceil(threads.max(1)).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = rows
            .chunks(per)
            .map(|slice| {
                let tok = tokenizer.clone();
                s.spawn(move || {
                    slice
                        .iter()
                        .map(|r| record(r, &tok))
                        .collect::<Result<Vec<_>>>()
                })
            })
            .collect();
        let mut out = Vec::with_capacity(rows.len());
        for h in handles {
            out.extend(
                h.join()
                    .map_err(|_| anyhow!("chunking thread panicked"))??,
            );
        }
        Ok(out)
    })
}

const INSERT_CASE: &str = "INSERT INTO cases(case_id, docket, gr_number, gr_key, title, case_name, date, \
    year, ponente, division, source_url, prefix) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)";
const INSERT_PASSAGE: &str =
    "INSERT INTO passages(case_rowid, ord, overlap, text) VALUES (?1,?2,?3,?4)";

fn insert(
    conn: &mut Connection,
    records: Vec<Option<Record>>,
    out: &mut CasesWritten,
) -> Result<()> {
    let tx = conn.transaction()?;
    {
        let (mut ins_case, mut ins_passage) =
            (tx.prepare(INSERT_CASE)?, tx.prepare(INSERT_PASSAGE)?);
        for rec in records {
            let Some(r) = rec else {
                out.skipped += 1;
                continue;
            };
            let gr = r.gr_key.as_deref().map(gr_display);
            ins_case.execute(params![
                r.case_id,
                r.docket,
                gr,
                r.gr_key,
                r.title,
                r.case_name,
                r.date,
                r.year,
                r.ponente,
                r.division,
                r.url,
                r.prefix
            ])?;
            let case_rowid = tx.last_insert_rowid();
            for (ord, part) in (0i64..).zip(&r.parts) {
                ins_passage.execute(params![
                    case_rowid,
                    ord,
                    i64::try_from(part.overlap)?,
                    part.text
                ])?;
            }
            out.cases += 1;
            out.passages += r.parts.len();
        }
    }
    tx.commit()?;
    Ok(())
}

/// Chunks and writes every decision row, `threads` at a time.
///
/// # Errors
/// When the tokenizer cannot be loaded or a write fails.
pub fn write(
    conn: &mut Connection,
    rows: &[Row],
    tokenizer_json: &Path,
    threads: usize,
) -> Result<CasesWritten> {
    let tokenizer = Tokenizer::from_file(tokenizer_json).map_err(|e| anyhow!(e.to_string()))?;
    let mut out = CasesWritten::default();
    for (i, batch) in rows.chunks(BATCH).enumerate() {
        let records = chunk_batch(batch, &tokenizer, threads)?;
        insert(conn, records, &mut out)?;
        eprintln!(
            "cases: {} decisions, {} passages ({} batches)",
            out.cases,
            out.passages,
            i + 1
        );
    }
    Ok(out)
}
