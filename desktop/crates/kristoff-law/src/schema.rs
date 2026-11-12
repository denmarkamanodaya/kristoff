//! The `phlaws.db` schema: the one source shared by the ingest tool and [`crate::LawIndex`].
//! Phase 3 §1.5.

/// DDL for a fresh database, run once by the ingest tool.
pub const CREATE: &str = "
CREATE TABLE laws (
    id INTEGER PRIMARY KEY,
    source_id TEXT NOT NULL UNIQUE,
    identifier TEXT NOT NULL,
    number INTEGER,
    is_constitution INTEGER NOT NULL,
    title TEXT NOT NULL,
    year INTEGER,
    status TEXT NOT NULL,
    source_url TEXT NOT NULL
);
CREATE INDEX laws_number ON laws(number);
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

/// Fills `chunks_fts` (contentless: only rowids and the index are stored; text lives in
/// `chunks` and `laws`).
pub const FILL_FTS: &str = "INSERT INTO chunks_fts(rowid, title, prefix, text) \
    SELECT c.id, l.title, c.prefix, c.text FROM chunks c JOIN laws l ON l.id = c.law_id";

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
    /// Number of laws.
    pub const LAW_COUNT: &str = "law_count";
    /// Number of chunks.
    pub const CHUNK_COUNT: &str = "chunk_count";
    /// Number of vectors.
    pub const VECTOR_COUNT: &str = "vector_count";
}

/// The `laws.source_id` of the 1987 Constitution in the dataset.
pub const CONSTITUTION_SOURCE_ID: &str = "ph-constitution-1987";
