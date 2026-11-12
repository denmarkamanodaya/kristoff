//! Parquet column names, fixed from the schema printed by `--schema` on 2026-11-12
//! (dataset revision 66ffcff9f6fa1288ca5247d7b20141dc841beb45). Never guess these.
//!
//! The dataset has no article order column: row order inside `articles.parquet` is the order.
//! It has no reliable date column either (`date` is null except for the Constitution), so the
//! year is read from the law text header `[ REPUBLIC ACT NO. n, Month dd, yyyy ]`.

/// `laws.parquet` columns.
pub mod law {
    /// Stable law id, e.g. `ph-republic-act-no-9262`.
    pub const ID: &str = "id";
    /// Display identifier, e.g. `Republic Act No. 9262`.
    pub const IDENTIFIER: &str = "identifier";
    /// Long title.
    pub const TITLE: &str = "title";
    /// Full text; its header carries the approval date.
    pub const TEXT: &str = "text";
    /// ISO date; set only for the Constitution.
    pub const DATE: &str = "date";
    /// `current` for every row in this revision.
    pub const STATUS: &str = "law_status";
    /// Source page.
    pub const SOURCE_URL: &str = "source_url";
}

/// `articles.parquet` columns.
pub mod article {
    /// Owning law id (`laws.id`).
    pub const LAW_ID: &str = "law_id";
    /// Article id; not unique (the Constitution repeats article headings).
    pub const ID: &str = "id";
    /// Article title.
    pub const TITLE: &str = "title";
    /// Label, e.g. `SEC. 5`, `SECTION 2`, `ARTICLE III`.
    pub const LABEL: &str = "article_number";
    /// Article text.
    pub const TEXT: &str = "text";
}

/// `OmniCorpus` `data/philippine_laws.parquet` columns (revision bb0626ce, schema printed
/// 2026-11-12: `text`, `citation_information`, `citations`, `url`, `label`). `citations` is empty in every
/// row and is not read.
pub mod omni {
    /// Full document text, no newlines.
    pub const TEXT: &str = "text";
    /// JSON object string; its `type` key marks the row type.
    pub const INFO: &str = "citation_information";
    /// `E-Library` page.
    pub const URL: &str = "url";
    /// `E-Library` shelf, e.g. `Republic Acts`.
    pub const LABEL: &str = "label";
}
