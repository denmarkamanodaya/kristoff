//! Reads the dataset parquet files into plain rows (Phase 3 §1.2).

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::Field;

use crate::columns::{article, law};

/// One law row.
pub struct LawRow {
    pub id: String,
    pub identifier: String,
    pub title: String,
    pub text: String,
    pub date: Option<String>,
    pub status: String,
    pub source_url: String,
}

/// One article row, in file order.
pub struct ArticleRow {
    pub law_id: String,
    pub id: String,
    pub title: String,
    pub label: String,
    pub text: String,
}

/// Returns the parquet schema of `path` as printed text.
pub fn schema(path: &Path) -> Result<String> {
    let reader = SerializedFileReader::new(File::open(path)?)?;
    let mut out = Vec::new();
    let meta = reader.metadata();
    parquet::schema::printer::print_schema(&mut out, meta.file_metadata().schema());
    Ok(format!(
        "{} rows\n{}",
        meta.file_metadata().num_rows(),
        String::from_utf8(out)?
    ))
}

fn rows(path: &Path, wanted: &[&str]) -> Result<Vec<HashMap<String, Option<String>>>> {
    let reader =
        SerializedFileReader::new(File::open(path).with_context(|| path.display().to_string())?)?;
    let mut out = Vec::new();
    for row in reader.get_row_iter(None)? {
        let row = row?;
        let mut map = HashMap::new();
        for (name, field) in row.get_column_iter() {
            if wanted.contains(&name.as_str()) {
                let value = match field {
                    Field::Str(s) => Some(s.clone()),
                    Field::Null => None,
                    other => Some(other.to_string()),
                };
                map.insert(name.clone(), value);
            }
        }
        if let Some(missing) = wanted.iter().find(|w| !map.contains_key(**w)) {
            return Err(anyhow!("{}: column {missing} not found", path.display()));
        }
        out.push(map);
    }
    Ok(out)
}

fn take(map: &mut HashMap<String, Option<String>>, key: &str) -> String {
    map.remove(key).flatten().unwrap_or_default()
}

/// Reads `laws.parquet`.
pub fn laws(path: &Path) -> Result<Vec<LawRow>> {
    let cols = [
        law::ID,
        law::IDENTIFIER,
        law::TITLE,
        law::TEXT,
        law::DATE,
        law::STATUS,
        law::SOURCE_URL,
    ];
    Ok(rows(path, &cols)?
        .into_iter()
        .map(|mut m| LawRow {
            id: take(&mut m, law::ID),
            identifier: take(&mut m, law::IDENTIFIER),
            title: take(&mut m, law::TITLE),
            text: take(&mut m, law::TEXT),
            date: m.remove(law::DATE).flatten(),
            status: take(&mut m, law::STATUS),
            source_url: take(&mut m, law::SOURCE_URL),
        })
        .collect())
}

/// Reads `articles.parquet` in file order.
pub fn articles(path: &Path) -> Result<Vec<ArticleRow>> {
    let cols = [
        article::LAW_ID,
        article::ID,
        article::TITLE,
        article::LABEL,
        article::TEXT,
    ];
    Ok(rows(path, &cols)?
        .into_iter()
        .map(|mut m| ArticleRow {
            law_id: take(&mut m, article::LAW_ID),
            id: take(&mut m, article::ID),
            title: take(&mut m, article::TITLE),
            label: take(&mut m, article::LABEL),
            text: take(&mut m, article::TEXT),
        })
        .collect())
}
