//! Turns cleaned rows into laws and chunks and writes them (Phase 3 §1.3–1.5).

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use kristoff_core::domain::law::reference::{LawRef, parse_references};
use kristoff_law::schema::{self, CONSTITUTION_SOURCE_ID, SOURCE_IPFS, kind};
use rusqlite::{Connection, params};

use crate::chunk;
use crate::clean::{self, CleanStats};
use crate::read::{ArticleRow, LawRow};

/// A law ready to insert.
pub struct Law {
    pub source_id: String,
    pub identifier: String,
    pub number: Option<u32>,
    pub is_constitution: bool,
    pub title: String,
    pub year: Option<u32>,
    pub status: String,
    pub source_url: String,
    /// `sources.id`.
    pub dataset_id: i64,
    /// `laws.kind` (`schema::kind`).
    pub kind: &'static str,
    /// Number within the kind, when it has one.
    pub kind_number: Option<u32>,
    /// Edition label (`schema::AS_ENACTED` for `OmniCorpus`), `None` for the current dataset.
    pub edition: Option<&'static str>,
}

/// A cleaned article, before chunking.
pub struct Article {
    pub label: String,
    pub text: String,
    pub truncated: bool,
    /// Rules of Court rule number, for `RULE n SEC. m` labels.
    pub rule: Option<u32>,
}

/// Reads the approval year from a law text header like `[ REPUBLIC ACT NO. 10, September 02, 1946 ]`.
pub fn header_year(text: &str) -> Option<u32> {
    let head: String = text.chars().take(1200).collect();
    let upper = head.to_uppercase();
    let start = upper
        .find("[ REPUBLIC ACT")
        .or_else(|| upper.find("[REPUBLIC ACT"))?;
    let end = upper[start..].find(']')? + start;
    upper[start..end]
        .split(|c: char| !c.is_ascii_digit())
        .filter(|t| t.len() == 4)
        .filter_map(|t| t.parse::<u32>().ok())
        .rfind(|y| (1900..=2100).contains(y))
}

fn law_from_row(row: &LawRow) -> Law {
    let is_constitution = row.id == CONSTITUTION_SOURCE_ID;
    let number = parse_references(&row.identifier)
        .laws
        .iter()
        .find_map(|l| match l {
            LawRef::RepublicAct(n) => Some(*n),
            _ => None,
        });
    let date_year = row
        .date
        .as_deref()
        .and_then(|d| d.get(..4))
        .and_then(|y| y.parse().ok());
    Law {
        source_id: row.id.clone(),
        identifier: clean::clean_line(&row.identifier),
        number,
        is_constitution,
        title: clean::clean_line(&row.title),
        year: date_year.or_else(|| header_year(&row.text)),
        status: row.status.clone(),
        source_url: row.source_url.clone(),
        dataset_id: SOURCE_IPFS,
        kind: if is_constitution {
            kind::CONSTITUTION
        } else {
            kind::RA
        },
        kind_number: number,
        edition: None,
    }
}

/// True for a bare Constitution article heading label such as `ARTICLE III`.
fn is_roman_article_heading(label: &str) -> bool {
    let refs = parse_references(label);
    label.to_uppercase().starts_with("ARTICLE")
        && refs.article().is_some()
        && refs.section().is_none()
        && !label.chars().any(|c| c.is_ascii_digit())
}

/// Cleans one law's articles in order; Constitution sections get their article in the label.
fn clean_articles(rows: &[&ArticleRow], stats: &mut CleanStats) -> Vec<Article> {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let longest = longest_text_per_id(rows);
    let mut current_article: Option<String> = None;
    let mut out = Vec::new();
    for row in rows {
        let text = clean::clean_text(&row.text, stats);
        if text.is_empty() {
            stats.empty_dropped += 1;
            continue;
        }
        if !seen.insert((row.id.clone(), text.clone())) {
            stats.duplicate_dropped += 1;
            continue;
        }
        let label = clean::clean_line(&row.label);
        if is_roman_article_heading(&label) {
            current_article = Some(label.clone());
        }
        let is_heading_twin = text == clean::clean_line(&row.title)
            && longest
                .get(row.id.as_str())
                .is_some_and(|l| *l > row.text.len());
        if is_heading_twin {
            stats.heading_dropped += 1;
            continue;
        }
        let truncated = clean::looks_truncated(&text);
        stats.truncated += usize::from(truncated);
        let label = with_article_context(current_article.as_deref(), label);
        out.push(Article {
            label,
            text,
            truncated,
            rule: None,
        });
    }
    out
}

/// Longest raw text per article id, to spot heading-only twins of a full article.
fn longest_text_per_id<'a>(rows: &[&'a ArticleRow]) -> HashMap<&'a str, usize> {
    rows.iter().fold(HashMap::new(), |mut m, r| {
        let e = m.entry(r.id.as_str()).or_insert(0);
        *e = (*e).max(r.text.len());
        m
    })
}

/// Prefixes a section label with the article heading it sits under (`ARTICLE III SECTION 2`).
fn with_article_context(article: Option<&str>, label: String) -> String {
    match article {
        Some(a) if parse_references(&label).section().is_some() => format!("{a} {label}"),
        _ => label,
    }
}

/// Counts written by [`write_laws_and_chunks`].
#[derive(Debug, Default)]
pub struct Written {
    pub laws: usize,
    pub chunks: usize,
    pub split_articles: usize,
}

/// The two dataset tables, as read.
pub struct Dataset<'a> {
    pub laws: &'a [LawRow],
    pub articles: &'a [ArticleRow],
}

/// Writes the current dataset's laws and chunks in one transaction (the FTS index is filled
/// once every source is in: `schema::FILL_FTS`).
pub fn write_laws_and_chunks(
    conn: &mut Connection,
    data: &Dataset<'_>,
    count: &dyn Fn(&str) -> Result<usize>,
    stats: &mut CleanStats,
) -> Result<Written> {
    let mut by_law: HashMap<&str, Vec<&ArticleRow>> = HashMap::new();
    for a in data.articles {
        by_law.entry(a.law_id.as_str()).or_default().push(a);
    }
    let tx = conn.transaction()?;
    let mut written = Written::default();
    {
        let mut ins_law = tx.prepare(INSERT_LAW)?;
        let mut ins_chunk = tx.prepare(INSERT_CHUNK)?;
        for row in data.laws {
            let law = law_from_row(row);
            insert_law(&mut ins_law, &law)?;
            written.laws += 1;
            let rows = by_law
                .get(row.id.as_str())
                .map(Vec::as_slice)
                .unwrap_or_default();
            let articles = clean_articles(rows, stats);
            let target = ChunkTarget {
                law_id: tx.last_insert_rowid(),
                law: &law,
                count,
            };
            insert_chunks(&mut ins_chunk, &target, &articles, &mut written)?;
        }
    }
    tx.commit()?;
    Ok(written)
}

pub const INSERT_LAW: &str = "INSERT INTO laws(source_id, identifier, number, is_constitution, \
    title, year, status, source_url, dataset_id, kind, kind_number, edition) \
    VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)";
pub const INSERT_CHUNK: &str = "INSERT INTO chunks(law_id, label, ord, prov, part, overlap, \
    sec_num, art_num, rule_num, prefix, text, truncated) \
    VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)";

pub fn insert_law(stmt: &mut rusqlite::Statement<'_>, law: &Law) -> Result<()> {
    stmt.execute(params![
        law.source_id,
        law.identifier,
        law.number,
        law.is_constitution,
        law.title,
        law.year,
        law.status,
        law.source_url,
        law.dataset_id,
        law.kind,
        law.kind_number,
        law.edition
    ])?;
    Ok(())
}

/// The law that chunks are written for, plus the token counter.
pub struct ChunkTarget<'a> {
    pub law_id: i64,
    pub law: &'a Law,
    pub count: &'a dyn Fn(&str) -> Result<usize>,
}

/// Chunks one law's articles and inserts them with consecutive `ord` values.
pub fn insert_chunks(
    stmt: &mut rusqlite::Statement<'_>,
    target: &ChunkTarget<'_>,
    articles: &[Article],
    written: &mut Written,
) -> Result<()> {
    let mut ord = 0u32;
    for (prov, article) in (0u32..).zip(articles) {
        let refs = parse_references(&article.label);
        let prefix = schema::chunk_prefix(&target.law.identifier, &article.label);
        let parts = chunk::split(&prefix, &article.text, target.count)?;
        written.split_articles += usize::from(parts.len() > 1);
        for (part, piece) in (0u32..).zip(parts.iter()) {
            stmt.execute(params![
                target.law_id,
                article.label,
                ord,
                prov,
                part,
                i64::try_from(piece.overlap)?,
                refs.section(),
                refs.article(),
                article.rule,
                prefix,
                piece.text,
                article.truncated
            ])?;
            ord += 1;
            written.chunks += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_year_from_the_senate_header() {
        let text = "\">\nH. No. 126 / 42 OG No. 10, 2368 (October, 1946)\n\n[ REPUBLIC ACT NO. 10, September 02, 1946 ]\n\nAN ACT";
        assert_eq!(header_year(text), Some(1946));
    }

    #[test]
    fn recognises_a_bare_roman_article_heading() {
        assert!(is_roman_article_heading("ARTICLE III"));
        assert!(!is_roman_article_heading("Article 73"));
        assert!(!is_roman_article_heading("SECTION 2"));
    }
}
