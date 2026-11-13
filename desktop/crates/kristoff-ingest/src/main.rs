//! `kristoff-ingest`: builds `phlaws.db` (and, with `OmniCorpus`, `phcases.db`). Developer tool;
//! never shipped in the app. Plans: docs/desktop/phases/phase-3-law-index.md §1 and
//! phase-7-omnicorpus.md.
//!
//! ```text
//! kristoff-ingest --source data/source --model data/model --out data/phlaws.db \
//!     [--omni data/source/omnicorpus/philippine_laws.parquet] [--cases-out data/phcases.db] \
//!     [--reuse-vectors old-phlaws.db] [--skip-laws] [--longest-first] [--batch 8] [--threads N] [--cpu]
//! kristoff-ingest --schema --source data/source
//! kristoff-ingest --omni-report data/source/omnicorpus/philippine_laws.parquet
//! kristoff-ingest --repair-case-names data/phcases.db
//! ```
//! Every stage is resumable: finished stages are skipped and embedding continues where it
//! stopped. Progress goes to stderr.

// This bin's only output channel is the terminal (progress, schema, final counts).
#![allow(clippy::print_stderr, clippy::print_stdout)]

mod build;
mod cases;
mod chunk;
mod clean;
mod columns;
mod db;
mod embed;
mod omni;
mod read;
mod stages;

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use kristoff_embed::{CandleEmbedder, DevicePreference};
use kristoff_law::{case_schema, schema};

/// What a run does.
#[derive(Default)]
enum Mode {
    /// Build the databases.
    #[default]
    Build,
    /// Print the dataset's parquet schema.
    Schema,
    /// Print the splitter report for the `OmniCorpus` codes.
    OmniReport(PathBuf),
    /// Re-apply the text repair to an existing `phcases.db`'s decision metadata.
    RepairCaseNames(PathBuf),
}

#[derive(Default)]
struct Args {
    source: PathBuf,
    model: PathBuf,
    out: PathBuf,
    omni: Option<PathBuf>,
    cases_out: Option<PathBuf>,
    reuse_vectors: Option<PathBuf>,
    mode: Mode,
    batch: usize,
    threads: usize,
    skip_laws: bool,
    longest_first: bool,
    cpu: bool,
}

fn parse_args() -> Result<Args> {
    let threads = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let mut a = Args {
        source: "data/source".into(),
        model: "data/model".into(),
        out: "data/phlaws.db".into(),
        batch: 8,
        threads,
        ..Args::default()
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| anyhow!("{flag} needs a value"));
        match flag.as_str() {
            "--source" => a.source = value()?.into(),
            "--model" => a.model = value()?.into(),
            "--out" => a.out = value()?.into(),
            "--omni" => a.omni = Some(value()?.into()),
            "--cases-out" => a.cases_out = Some(value()?.into()),
            "--reuse-vectors" => a.reuse_vectors = Some(value()?.into()),
            "--omni-report" => a.mode = Mode::OmniReport(value()?.into()),
            "--repair-case-names" => a.mode = Mode::RepairCaseNames(value()?.into()),
            "--batch" => a.batch = value()?.parse()?,
            "--threads" => a.threads = value()?.parse()?,
            "--schema" => a.mode = Mode::Schema,
            "--skip-laws" => a.skip_laws = true,
            "--longest-first" => a.longest_first = true,
            "--cpu" => a.cpu = true,
            other => bail!("unknown flag {other}"),
        }
    }
    Ok(a)
}

fn laws(args: &Args, inp: &stages::Inputs<'_>, embedder: &CandleEmbedder) -> Result<()> {
    let mut conn = db::open(&args.out, schema::CREATE)?;
    stages::sources(&conn, inp)?;
    stages::ipfs_chunks(&mut conn, inp, embedder)?;
    stages::omni_statutes(&mut conn, inp, embedder)?;
    stages::law_fts(&mut conn)?;
    if let Some(prev) = &args.reuse_vectors {
        eprintln!(
            "reused {} vectors from {}",
            stages::reuse_vectors(&conn, prev, inp.model_rev)?,
            prev.display()
        );
    }
    stages::law_embed(&mut conn, embedder, args.batch)?;
    stages::law_finish(&conn, inp.model_rev, &args.out)
}

fn cases(
    out: &Path,
    args: &Args,
    inp: &stages::Inputs<'_>,
    embedder: &CandleEmbedder,
) -> Result<()> {
    let mut conn = db::open(out, case_schema::CREATE)?;
    stages::case_rows(&mut conn, inp)?;
    stages::case_embed(&mut conn, embedder, args.batch, args.longest_first)?;
    stages::case_finish(&conn, inp.model_rev, out)
}

fn main() -> Result<()> {
    let args = parse_args()?;
    match &args.mode {
        Mode::OmniReport(parquet) => {
            omni::report::lines(parquet)?
                .iter()
                .for_each(|l| println!("{l}"));
            Ok(())
        }
        Mode::Schema => {
            for f in ["laws.parquet", "articles.parquet"] {
                println!("== {f}\n{}", read::schema(&args.source.join(f))?);
            }
            Ok(())
        }
        Mode::RepairCaseNames(db) => {
            println!("repaired case metadata rows: {}", cases::repair::run(db)?);
            Ok(())
        }
        Mode::Build => build_all(&args),
    }
}

fn build_all(args: &Args) -> Result<()> {
    let data_dir = args.source.parent().unwrap_or_else(|| Path::new("."));
    let dataset_rev = db::lock_revision(&data_dir.join("dataset.lock"))?;
    let model_rev = db::lock_revision(&data_dir.join("model.lock"))?;
    let omni_lock = data_dir.join("omnicorpus.lock");
    let omni_rev = args
        .omni
        .as_ref()
        .map(|_| db::lock_revision(&omni_lock))
        .transpose()?;
    let tokenizer_json = args.model.join("tokenizer.json");
    let inp = stages::Inputs {
        source: &args.source,
        omni: args.omni.as_deref(),
        tokenizer_json: &tokenizer_json,
        dataset_rev: &dataset_rev,
        omni_rev: omni_rev.as_deref(),
        model_rev: &model_rev,
        threads: args.threads,
    };
    let pref = if args.cpu {
        DevicePreference::Cpu
    } else {
        DevicePreference::MetalThenCpu
    };
    let embedder = CandleEmbedder::load(&args.model, pref).map_err(|e| anyhow!(e.to_string()))?;
    if !args.skip_laws {
        laws(args, &inp, &embedder)?;
    }
    match &args.cases_out {
        Some(out) => cases(out, args, &inp, &embedder),
        None => Ok(()),
    }
}
