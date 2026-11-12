//! `kristoff-ingest`: builds `phlaws.db` from the PH laws dataset (Phase 3 §1).
//!
//! ```text
//! kristoff-ingest --source data/source --model data/model --out data/phlaws.db [--batch 8]
//! kristoff-ingest --schema --source data/source
//! ```
//! Stages: read + clean + chunk + FTS (skipped when `meta.chunks_stage = done`), embed
//! (resumable: skips chunks that already have vectors), finish (meta counts, `VACUUM`,
//! `journal_mode=DELETE`). Progress goes to stderr. Developer tool; never shipped in the app.

// This bin's only output channel is the terminal (progress, schema, final counts).
#![allow(clippy::print_stderr, clippy::print_stdout)]

mod build;
mod chunk;
mod clean;
mod columns;
mod read;

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result, anyhow, bail};
use kristoff_core::util::time::utc_day;
use kristoff_embed::{CandleEmbedder, DevicePreference};
use kristoff_law::quant::quantize;
use kristoff_law::schema::{self, meta_key};
use rusqlite::{Connection, OptionalExtension, params};

struct Args {
    source: PathBuf,
    model: PathBuf,
    out: PathBuf,
    batch: usize,
    schema_only: bool,
    cpu: bool,
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        source: PathBuf::from("data/source"),
        model: PathBuf::from("data/model"),
        out: PathBuf::from("data/phlaws.db"),
        batch: 8,
        schema_only: false,
        cpu: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| anyhow!("{flag} needs a value"));
        match flag.as_str() {
            "--source" => args.source = value()?.into(),
            "--model" => args.model = value()?.into(),
            "--out" => args.out = value()?.into(),
            "--batch" => args.batch = value()?.parse()?,
            "--schema" => args.schema_only = true,
            "--cpu" => args.cpu = true,
            other => bail!("unknown flag {other}"),
        }
    }
    Ok(args)
}

/// Reads `# revision: <sha>` from a lock file.
fn lock_revision(path: &Path) -> Result<String> {
    let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    text.lines()
        .find_map(|l| l.strip_prefix("# revision: "))
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("{}: no revision line", path.display()))
}

fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
        .optional()?)
}

fn open_db(path: &Path) -> Result<Connection> {
    let fresh = !path.exists();
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    if fresh {
        conn.execute_batch(schema::CREATE)?;
    }
    Ok(conn)
}

fn stage_chunks(
    conn: &mut Connection,
    args: &Args,
    embedder: &CandleEmbedder,
    dataset_rev: &str,
) -> Result<()> {
    if get_meta(conn, meta_key::CHUNKS_STAGE)?.as_deref() == Some("done") {
        eprintln!("chunks: already built, skipping");
        return Ok(());
    }
    let t = Instant::now();
    let laws = read::laws(&args.source.join("laws.parquet"))?;
    let articles = read::articles(&args.source.join("articles.parquet"))?;
    eprintln!(
        "read: {} laws, {} articles in {:.1}s",
        laws.len(),
        articles.len(),
        t.elapsed().as_secs_f64()
    );
    let mut stats = clean::CleanStats::default();
    let count = |s: &str| embedder.token_count(s).map_err(|e| anyhow!(e.to_string()));
    let written = build::write_laws_and_chunks(
        conn,
        &build::Dataset {
            laws: &laws,
            articles: &articles,
        },
        &count,
        &mut stats,
    )?;
    eprintln!("clean: {stats:?}");
    eprintln!(
        "chunks: {} laws, {} chunks, {} articles split, in {:.1}s",
        written.laws,
        written.chunks,
        written.split_articles,
        t.elapsed().as_secs_f64()
    );
    set_meta(conn, meta_key::DATASET_REVISION, dataset_rev)?;
    set_meta(conn, meta_key::CHUNKS_STAGE, "done")?;
    Ok(())
}

fn stage_embed(conn: &mut Connection, embedder: &CandleEmbedder, batch: usize) -> Result<usize> {
    let mut pending: Vec<(i64, String)> = conn
        .prepare("SELECT c.id, c.prefix || c.text FROM chunks c LEFT JOIN vectors v ON v.chunk_id = c.id WHERE v.chunk_id IS NULL")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    // Similar lengths per batch keep padding (and wasted attention) low.
    pending.sort_by_key(|(_, text)| text.len());
    let total = pending.len();
    eprintln!(
        "embed: {total} chunks pending on {}, batch {batch}",
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
            let mut ins =
                tx.prepare_cached("INSERT INTO vectors(chunk_id, scale, v) VALUES (?1, ?2, ?3)")?;
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
            eprintln!("embed: {done}/{total} ({rate:.1}/s)");
        }
    }
    Ok(total)
}

fn count(conn: &Connection, table: &str) -> Result<i64> {
    Ok(conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?)
}

fn stage_finish(conn: &Connection, model_rev: &str, out: &Path) -> Result<()> {
    let (laws, chunks, vectors) = (
        count(conn, "laws")?,
        count(conn, "chunks")?,
        count(conn, "vectors")?,
    );
    if vectors != chunks {
        bail!("{vectors} vectors for {chunks} chunks; re-run to resume embedding");
    }
    set_meta(conn, meta_key::MODEL_REVISION, model_rev)?;
    set_meta(conn, meta_key::BUILD_DATE, &build_date()?)?;
    set_meta(conn, meta_key::LAW_COUNT, &laws.to_string())?;
    set_meta(conn, meta_key::CHUNK_COUNT, &chunks.to_string())?;
    set_meta(conn, meta_key::VECTOR_COUNT, &vectors.to_string())?;
    conn.execute_batch("INSERT INTO chunks_fts(chunks_fts) VALUES('optimize');")?;
    conn.pragma_update(None, "journal_mode", "DELETE")?;
    conn.execute_batch("VACUUM;")?;
    let size = std::fs::metadata(out)?.len();
    println!("phlaws.db: {size} bytes, {laws} laws, {chunks} chunks, {vectors} vectors");
    Ok(())
}

/// Today's UTC date `yyyy-mm-dd` from the system clock (the bin is the composition root).
fn build_date() -> Result<String> {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();
    Ok(utc_day(i64::try_from(ms)?))
}

fn main() -> Result<()> {
    let args = parse_args()?;
    if args.schema_only {
        for f in ["laws.parquet", "articles.parquet"] {
            println!("== {f}\n{}", read::schema(&args.source.join(f))?);
        }
        return Ok(());
    }
    let data_dir = args.source.parent().unwrap_or_else(|| Path::new("."));
    let dataset_rev = lock_revision(&data_dir.join("dataset.lock"))?;
    let model_rev = lock_revision(&data_dir.join("model.lock"))?;
    let pref = if args.cpu {
        DevicePreference::Cpu
    } else {
        DevicePreference::MetalThenCpu
    };
    let embedder = CandleEmbedder::load(&args.model, pref).map_err(|e| anyhow!(e.to_string()))?;
    let mut conn = open_db(&args.out)?;
    stage_chunks(&mut conn, &args, &embedder, &dataset_rev)?;
    stage_embed(&mut conn, &embedder, args.batch)?;
    stage_finish(&conn, &model_rev, &args.out)
}
