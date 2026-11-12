//! The `phcases.db` schema (Phase 7, owner ruling D-20): Supreme Court decisions chunked fully
//! into 480-token passages, each with FTS and its own e5 vector. The one source for the ingest
//! tool and `CaseIndex`.

/// DDL for a fresh `phcases.db`.
pub const CREATE: &str = "
CREATE TABLE cases (
    id INTEGER PRIMARY KEY,
    case_id TEXT NOT NULL UNIQUE,
    docket TEXT,
    gr_number TEXT,
    gr_key TEXT,
    title TEXT NOT NULL,
    case_name TEXT NOT NULL,
    date TEXT NOT NULL,
    year INTEGER NOT NULL,
    ponente TEXT,
    division TEXT,
    source_url TEXT NOT NULL,
    prefix TEXT NOT NULL
);
CREATE INDEX cases_gr ON cases(gr_key);
CREATE INDEX cases_year ON cases(year);
CREATE TABLE passages (
    id INTEGER PRIMARY KEY,
    case_rowid INTEGER NOT NULL REFERENCES cases(id),
    ord INTEGER NOT NULL,
    overlap INTEGER NOT NULL,
    text TEXT NOT NULL
);
CREATE UNIQUE INDEX passages_case_ord ON passages(case_rowid, ord);
CREATE VIRTUAL TABLE passages_fts USING fts5(
    title, text, content='',
    tokenize='porter unicode61 remove_diacritics 2'
);
CREATE TABLE passage_vectors (
    passage_id INTEGER PRIMARY KEY REFERENCES passages(id),
    scale REAL NOT NULL,
    v BLOB NOT NULL
);
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
";

/// Fills `passages_fts`; the title column carries the short title and docket of the case.
pub const FILL_FTS: &str = "INSERT INTO passages_fts(rowid, title, text) \
    SELECT p.id, c.title || ' ' || coalesce(c.docket, ''), p.text \
    FROM passages p JOIN cases c ON c.id = p.case_rowid";

/// BM25 over (title, text). Both weigh 1.0: party names in the title are what case-name
/// questions ask for, and the text carries everything else.
pub const FTS_RANK: &str = "bm25(passages_fts, 1.0, 1.0)";

/// Builds a decision's passage prefix `"<short title> · <docket> (<year>)\n"`, repeated on every
/// embedded passage (as `schema::chunk_prefix` does for statutes).
#[must_use]
pub fn passage_prefix(title: &str, docket: Option<&str>, year: i64) -> String {
    match docket {
        Some(d) => format!("{title} \u{b7} {d} ({year})\n"),
        None => format!("{title} ({year})\n"),
    }
}

/// `meta` keys of `phcases.db`.
pub mod meta_key {
    /// `OmniCorpus` revision.
    pub const OMNI_REVISION: &str = "omni_revision";
    /// Model revision of the vectors.
    pub const MODEL_REVISION: &str = "model_revision";
    /// Set to `done` once cases and passages are written.
    pub const CASES_STAGE: &str = "cases_stage";
    /// Number of decisions.
    pub const CASE_COUNT: &str = "case_count";
    /// Number of passages.
    pub const PASSAGE_COUNT: &str = "passage_count";
    /// Set to `done` once `passages_fts` is filled.
    pub const FTS_STAGE: &str = "fts_stage";
    /// Build date.
    pub const BUILD_DATE: &str = "build_date";
}
