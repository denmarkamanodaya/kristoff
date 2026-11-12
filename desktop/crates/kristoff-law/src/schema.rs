//! The `phlaws.db` schema: the one source shared by the ingest tool and [`crate::LawIndex`].
//! Phase 3 §1.5; Phase 7 adds the `sources` table, law kinds and editions.

/// DDL for a fresh database, run once by the ingest tool.
pub const CREATE: &str = "
CREATE TABLE sources (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    revision TEXT NOT NULL,
    licence TEXT NOT NULL,
    attribution TEXT NOT NULL
);
CREATE TABLE laws (
    id INTEGER PRIMARY KEY,
    source_id TEXT NOT NULL UNIQUE,
    identifier TEXT NOT NULL,
    number INTEGER,
    is_constitution INTEGER NOT NULL,
    title TEXT NOT NULL,
    year INTEGER,
    status TEXT NOT NULL,
    source_url TEXT NOT NULL,
    dataset_id INTEGER NOT NULL REFERENCES sources(id),
    kind TEXT NOT NULL,
    kind_number INTEGER,
    edition TEXT
);
CREATE INDEX laws_number ON laws(number);
CREATE INDEX laws_kind ON laws(kind, kind_number);
CREATE TABLE chunks (
    id INTEGER PRIMARY KEY,
    law_id INTEGER NOT NULL REFERENCES laws(id),
    label TEXT NOT NULL,
    ord INTEGER NOT NULL,
    prov INTEGER NOT NULL,
    part INTEGER NOT NULL,
    overlap INTEGER NOT NULL,
    sec_num INTEGER,
    art_num INTEGER,
    rule_num INTEGER,
    prefix TEXT NOT NULL,
    text TEXT NOT NULL,
    truncated INTEGER NOT NULL
);
CREATE UNIQUE INDEX chunks_law_ord ON chunks(law_id, ord);
CREATE INDEX chunks_ref ON chunks(law_id, sec_num, art_num);
CREATE INDEX chunks_prov ON chunks(law_id, prov, part);
CREATE VIRTUAL TABLE chunks_fts USING fts5(
    title, prefix, text, content='',
    tokenize='porter unicode61 remove_diacritics 2'
);
CREATE TABLE vectors (
    chunk_id INTEGER PRIMARY KEY REFERENCES chunks(id),
    scale REAL NOT NULL,
    v BLOB NOT NULL
);
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
";

/// Adds chunks with `id > ?1` to `chunks_fts` (contentless: only rowids and the index are
/// stored; text lives in `chunks` and `laws`). Chunk ids only grow, so a re-run after new
/// sources were added indexes just the new chunks.
pub const FILL_FTS: &str = "INSERT INTO chunks_fts(rowid, title, prefix, text) \
    SELECT c.id, l.title, c.prefix, c.text FROM chunks c JOIN laws l ON l.id = c.law_id WHERE c.id > ?1";

/// BM25 rank over `chunks_fts` columns (title, prefix, text). The title is shared by every
/// chunk of a law, so it says which law but never which provision: weight 0.5, below the text,
/// lets title words find the law without outranking provision-level matches. The prefix
/// (identifier + label) names the provision itself, so it gets 2.0.
pub const FTS_RANK: &str = "bm25(chunks_fts, 0.5, 2.0, 1.0)";

/// Builds the chunk prefix `"<identifier> · <label>\n"`. The law title is left out (lead
/// decision 2026-11-12): it is the same for every chunk of a law, so in the embedded text it
/// drowned out the provision. The title is indexed in its own FTS column instead.
#[must_use]
pub fn chunk_prefix(identifier: &str, label: &str) -> String {
    format!("{identifier} · {label}\n")
}

/// Rows of the `sources` table: `(id, name, licence, attribution)`. The revision comes from
/// the dataset's lock file at ingest time. `name` is what `LawHit::source` carries.
pub const SOURCES: [(i64, &str, &str, &str); 2] = [
    (
        1,
        "ipfs_philippines_laws",
        "dataset terms of endomorphosis/ipfs_philippines_laws",
        "endomorphosis/ipfs_philippines_laws (Hugging Face)",
    ),
    (
        2,
        "omnicorpus",
        "ODC-By-1.0",
        "Ramos Jr., D. B. (2024). Philippine OmniCorpus. mongramosjr/philippine-omnicorpus (Hugging Face)",
    ),
];

/// `sources.id` of the current dataset (Republic Acts and the 1987 Constitution).
pub const SOURCE_IPFS: i64 = 1;
/// `sources.id` of `OmniCorpus` (codes, old statutes, the RAs the current dataset lacks).
pub const SOURCE_OMNI: i64 = 2;

/// `laws.edition` of `OmniCorpus` codes and legislative statutes (RA, Act, CA, BP, PD and the
/// Family Code): the text as passed or decreed, later amendments not folded in.
pub const AS_ENACTED: &str = "As enacted";
/// `laws.edition` of `OmniCorpus` Rules of Court documents (the 2019 A.M.s are not merged in).
pub const AS_PROMULGATED: &str = "As promulgated";
/// `laws.edition` of `OmniCorpus` executive issuances, LOIs and General Orders; amendments and
/// revocations are not tracked.
pub const AS_ISSUED: &str = "As issued";
/// `laws.edition` of `OmniCorpus` treaties.
pub const AS_SIGNED: &str = "As signed";
/// `laws.edition` of earlier constitutions and the 1934-35 `ConCon` / 1986 `ConCom` records.
pub const CONSTITUTIONAL_RECORDS: &str = "Constitutional records";
/// `laws.edition` of judiciary benchbooks and manuals.
pub const COURT_REFERENCE: &str = "Court reference";

/// `laws.kind` values (one source for the ingest tool and the index).
pub mod kind {
    /// Republic Act.
    pub const RA: &str = "RA";
    /// The 1987 Constitution.
    pub const CONSTITUTION: &str = "CONSTITUTION";
    /// Act of the Philippine Commission or Legislature (e.g. Act No. 3815, the RPC).
    pub const ACT: &str = "ACT";
    /// Commonwealth Act.
    pub const CA: &str = "CA";
    /// Batas Pambansa.
    pub const BP: &str = "BP";
    /// Presidential Decree.
    pub const PD: &str = "PD";
    /// Executive Order.
    pub const EO: &str = "EO";
    /// Presidential Proclamation.
    pub const PROC: &str = "PROC";
    /// Administrative Order.
    pub const AO: &str = "AO";
    /// Memorandum Order.
    pub const MO: &str = "MO";
    /// Memorandum Circular.
    pub const MC: &str = "MC";
    /// Agency issuance from the National Administrative Register.
    pub const NAR: &str = "NAR";
    /// Letter of Implementation.
    pub const LOIMP: &str = "LOIMP";
    /// Letter of Instructions.
    pub const LOI: &str = "LOI";
    /// General Order.
    pub const GO: &str = "GO";
    /// Rules of Court documents.
    pub const RULES: &str = "RULES";
    /// Treaty or international agreement.
    pub const TREATY: &str = "TREATY";
    /// Earlier constitution or constitutional convention/commission record.
    pub const CONREC: &str = "CONREC";
    /// Judiciary benchbook or manual.
    pub const COURTREF: &str = "COURTREF";
}

/// `meta` keys.
pub mod meta_key {
    /// Dataset revision the db was built from.
    pub const DATASET_REVISION: &str = "dataset_revision";
    /// Model revision the vectors were built with.
    pub const MODEL_REVISION: &str = "model_revision";
    /// Build date, `yyyy-mm-dd`.
    pub const BUILD_DATE: &str = "build_date";
    /// Set to `done` once laws and chunks are written; embedding resumes after it.
    pub const CHUNKS_STAGE: &str = "chunks_stage";
    /// Set to `done` once the `OmniCorpus` statutes are written.
    pub const OMNI_STAGE: &str = "omni_stage";
    /// `OmniCorpus` revision the statutes came from.
    pub const OMNI_REVISION: &str = "omni_revision";
    /// Highest chunk id already in `chunks_fts`.
    pub const FTS_MAX_CHUNK: &str = "fts_max_chunk";
    /// Number of laws.
    pub const LAW_COUNT: &str = "law_count";
    /// Number of chunks.
    pub const CHUNK_COUNT: &str = "chunk_count";
    /// Number of vectors.
    pub const VECTOR_COUNT: &str = "vector_count";
}

/// The `laws.source_id` of the 1987 Constitution in the dataset.
pub const CONSTITUTION_SOURCE_ID: &str = "ph-constitution-1987";
