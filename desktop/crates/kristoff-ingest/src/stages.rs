//! The build stages of `phlaws.db` and `phcases.db`. Each stage marks itself done in `meta`, so a
//! re-run skips finished stages and embedding resumes where it stopped.

use std::path::Path;
use std::time::Instant;

use anyhow::{Result, anyhow, bail};
use kristoff_embed::CandleEmbedder;
use kristoff_law::case_schema;
use kristoff_law::schema::{self, SOURCES, meta_key};
use rusqlite::{Connection, params};

use crate::db::{build_date, compact, count, get_meta, set_meta, stage_done};
use crate::{build, cases, clean, embed, omni, read};

/// Inputs every stage may need.
pub struct Inputs<'a> {
    pub source: &'a Path,
    pub omni: Option<&'a Path>,
    pub tokenizer_json: &'a Path,
    pub dataset_rev: &'a str,
    pub omni_rev: Option<&'a str>,
    pub model_rev: &'a str,
    pub threads: usize,
}

fn token_counter(embedder: &CandleEmbedder) -> impl Fn(&str) -> Result<usize> + '_ {
    |s: &str| embedder.token_count(s).map_err(|e| anyhow!(e.to_string()))
}

/// Writes the `sources` rows (revision per source; the `OmniCorpus` row only when it is used).
pub fn sources(conn: &Connection, inp: &Inputs<'_>) -> Result<()> {
    for (id, name, licence, attribution) in SOURCES {
        let revision = if id == schema::SOURCE_IPFS {
            Some(inp.dataset_rev)
        } else {
            inp.omni_rev
        };
        if let Some(rev) = revision {
            conn.execute(
                "INSERT OR REPLACE INTO sources(id, name, revision, licence, attribution) VALUES (?1,?2,?3,?4,?5)",
                params![id, name, rev, licence, attribution],
            )?;
        }
    }
    Ok(())
}

/// The current dataset's laws and chunks.
pub fn ipfs_chunks(
    conn: &mut Connection,
    inp: &Inputs<'_>,
    embedder: &CandleEmbedder,
) -> Result<()> {
    if stage_done(conn, meta_key::CHUNKS_STAGE)? {
        return Ok(());
    }
    let t = Instant::now();
    let laws = read::laws(&inp.source.join("laws.parquet"))?;
    let articles = read::articles(&inp.source.join("articles.parquet"))?;
    let mut stats = clean::CleanStats::default();
    let data = build::Dataset {
        laws: &laws,
        articles: &articles,
    };
    let w = build::write_laws_and_chunks(conn, &data, &token_counter(embedder), &mut stats)?;
    eprintln!("clean: {stats:?}");
    eprintln!(
        "ipfs: {} laws, {} chunks, {} split, {:.0}s",
        w.laws,
        w.chunks,
        w.split_articles,
        t.elapsed().as_secs_f64()
    );
    set_meta(conn, meta_key::DATASET_REVISION, inp.dataset_rev)?;
    set_meta(conn, meta_key::CHUNKS_STAGE, "done")
}

/// `OmniCorpus` statutes (skipped when no `OmniCorpus` file is given).
pub fn omni_statutes(
    conn: &mut Connection,
    inp: &Inputs<'_>,
    embedder: &CandleEmbedder,
) -> Result<()> {
    let (Some(path), Some(rev)) = (inp.omni, inp.omni_rev) else {
        return Ok(());
    };
    // Always runs: it adds only laws not yet written, so a widened selection is picked up.
    let t = Instant::now();
    let mut excluded: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let rows = omni::read::rows(path, |info, label| {
        if let Some(reason) = omni::select::exclusion(info, label) {
            *excluded.entry(reason).or_default() += 1;
        }
        omni::select::statute(info, label).is_some()
    })?;
    for (reason, n) in &excluded {
        eprintln!("omni excluded: {n} rows: {reason}");
    }
    let w = omni::statutes::write(conn, &rows, &token_counter(embedder))?;
    eprintln!(
        "omni statutes: {} laws, {} chunks, {} split, {} duplicate RAs skipped, {} mojibake repaired, {:.0}s",
        w.written.laws,
        w.written.chunks,
        w.written.split_articles,
        w.duplicate_ras,
        w.mojibake_fixed,
        t.elapsed().as_secs_f64()
    );
    set_meta(conn, meta_key::OMNI_REVISION, rev)?;
    set_meta(conn, meta_key::OMNI_STAGE, "done")
}

/// Adds every chunk not yet indexed to `chunks_fts`.
pub fn law_fts(conn: &mut Connection) -> Result<()> {
    let done: i64 = get_meta(conn, meta_key::FTS_MAX_CHUNK)?.map_or(Ok(0), |v| v.parse())?;
    let max: i64 = conn.query_row("SELECT coalesce(max(id), 0) FROM chunks", [], |r| r.get(0))?;
    let tx = conn.transaction()?;
    tx.execute(schema::FILL_FTS, [done])?;
    tx.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES (?1, ?2)",
        params![meta_key::FTS_MAX_CHUNK, max.to_string()],
    )?;
    tx.commit()?;
    Ok(())
}

/// Copies vectors from an earlier build for chunks whose prefix and text are unchanged (same
/// model, same input, same vector), so only new chunks are embedded.
pub fn reuse_vectors(conn: &Connection, previous: &Path, model_rev: &str) -> Result<usize> {
    let uri = format!("file:{}?immutable=1", previous.display());
    conn.execute("ATTACH DATABASE ?1 AS prev", [uri])?;
    let prev_model: Option<String> = conn
        .query_row(
            "SELECT value FROM prev.meta WHERE key = ?1",
            [meta_key::MODEL_REVISION],
            |r| r.get(0),
        )
        .ok();
    if prev_model.as_deref() != Some(model_rev) {
        conn.execute_batch("DETACH DATABASE prev")?;
        bail!(
            "{}: built with another model ({prev_model:?}); not reusing",
            previous.display()
        );
    }
    let n = conn.execute(
        "INSERT OR IGNORE INTO vectors(chunk_id, scale, v) SELECT c.id, pv.scale, pv.v FROM chunks c \
         JOIN prev.chunks pc ON pc.prefix = c.prefix AND pc.text = c.text JOIN prev.vectors pv ON pv.chunk_id = pc.id",
        [],
    )?;
    conn.execute_batch("DETACH DATABASE prev")?;
    Ok(n)
}

/// Embeds every statute chunk without a vector.
pub fn law_embed(conn: &mut Connection, embedder: &CandleEmbedder, batch: usize) -> Result<usize> {
    let target = embed::Target {
        what: "law chunks",
        pending_sql: "SELECT c.id, c.prefix || c.text FROM chunks c LEFT JOIN vectors v ON v.chunk_id = c.id WHERE v.chunk_id IS NULL",
        insert_sql: "INSERT OR IGNORE INTO vectors(chunk_id, scale, v) VALUES (?1, ?2, ?3)",
        longest_first: false,
    };
    embed::run(conn, embedder, batch, &target)
}

/// Meta counts, FTS optimize and compaction of `phlaws.db`; prints its size.
pub fn law_finish(conn: &Connection, model_rev: &str, out: &Path) -> Result<()> {
    let (laws, chunks, vectors) = (
        count(conn, "laws")?,
        count(conn, "chunks")?,
        count(conn, "vectors")?,
    );
    if vectors != chunks {
        bail!("{vectors} vectors for {chunks} chunks; re-run to resume embedding");
    }
    for (k, v) in [
        (meta_key::MODEL_REVISION, model_rev.to_owned()),
        (meta_key::BUILD_DATE, build_date()?),
        (meta_key::LAW_COUNT, laws.to_string()),
        (meta_key::CHUNK_COUNT, chunks.to_string()),
        (meta_key::VECTOR_COUNT, vectors.to_string()),
    ] {
        set_meta(conn, k, &v)?;
    }
    conn.execute_batch("INSERT INTO chunks_fts(chunks_fts) VALUES('optimize');")?;
    let size = compact(conn, out)?;
    println!("phlaws.db: {size} bytes, {laws} laws, {chunks} chunks, {vectors} vectors");
    Ok(())
}

/// Decisions, passages and FTS of `phcases.db`.
pub fn case_rows(conn: &mut Connection, inp: &Inputs<'_>) -> Result<()> {
    let (Some(path), Some(rev)) = (inp.omni, inp.omni_rev) else {
        bail!("--cases-out needs --omni")
    };
    if !stage_done(conn, case_schema::meta_key::CASES_STAGE)? {
        let t = Instant::now();
        let rows = omni::read::rows(path, |info, _| omni::select::is_decision(info))?;
        let w = cases::build::write(conn, &rows, inp.tokenizer_json, inp.threads)?;
        eprintln!("cases: {w:?} in {:.0}s", t.elapsed().as_secs_f64());
        set_meta(conn, case_schema::meta_key::OMNI_REVISION, rev)?;
        set_meta(conn, case_schema::meta_key::CASES_STAGE, "done")?;
    }
    if !stage_done(conn, case_schema::meta_key::FTS_STAGE)? {
        conn.execute_batch(&format!("BEGIN; {}; COMMIT;", case_schema::FILL_FTS))?;
        set_meta(conn, case_schema::meta_key::FTS_STAGE, "done")?;
    }
    Ok(())
}

/// Embeds every passage without a vector.
pub fn case_embed(
    conn: &mut Connection,
    embedder: &CandleEmbedder,
    batch: usize,
    longest_first: bool,
) -> Result<usize> {
    let target = embed::Target {
        what: "decision passages",
        pending_sql: "SELECT p.id, c.prefix || p.text FROM passages p JOIN cases c ON c.id = p.case_rowid \
                      LEFT JOIN passage_vectors v ON v.passage_id = p.id WHERE v.passage_id IS NULL",
        insert_sql: "INSERT OR IGNORE INTO passage_vectors(passage_id, scale, v) VALUES (?1, ?2, ?3)",
        longest_first,
    };
    embed::run(conn, embedder, batch, &target)
}

/// Meta counts, FTS optimize and compaction of `phcases.db`; prints its size.
pub fn case_finish(conn: &Connection, model_rev: &str, out: &Path) -> Result<()> {
    let (cases, passages, vectors) = (
        count(conn, "cases")?,
        count(conn, "passages")?,
        count(conn, "passage_vectors")?,
    );
    if vectors != passages {
        bail!("{vectors} vectors for {passages} passages; re-run to resume embedding");
    }
    for (key, v) in [
        (case_schema::meta_key::MODEL_REVISION, model_rev.to_owned()),
        (case_schema::meta_key::BUILD_DATE, build_date()?),
        (case_schema::meta_key::CASE_COUNT, cases.to_string()),
        (case_schema::meta_key::PASSAGE_COUNT, passages.to_string()),
    ] {
        set_meta(conn, key, &v)?;
    }
    conn.execute_batch("INSERT INTO passages_fts(passages_fts) VALUES('optimize');")?;
    let size = compact(conn, out)?;
    println!("phcases.db: {size} bytes, {cases} decisions, {passages} passages, {vectors} vectors");
    Ok(())
}
