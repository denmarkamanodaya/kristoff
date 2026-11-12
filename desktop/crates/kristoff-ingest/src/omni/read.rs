//! Reads `OmniCorpus` rows (Phase 7). Column names live in `columns::omni`.

use std::path::Path;

use anyhow::{Result, anyhow};
use serde::Deserialize;

use crate::columns::omni;
use crate::read::{field_text, for_each_row};

/// The keys of `citation_information` this ingest reads; each type uses a subset.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Info {
    /// Row type: `republic_act`, `court_decision`, `act`, …
    #[serde(rename = "type")]
    pub kind: String,
    /// e.g. `Act No. 3815`.
    pub short_title: Option<String>,
    /// Long title.
    pub title: Option<String>,
    /// Citation line (decisions: parties, G.R. No., court, year).
    pub citation: Option<String>,
    /// Decisions: full case name.
    pub case_name: Option<String>,
    /// Decisions and other documents: `yyyy-mm-dd`.
    pub date: Option<String>,
    /// Statutes.
    pub date_of_enactment: Option<String>,
    /// Decrees and issuances.
    pub date_of_issuance: Option<String>,
    /// Rules of Court.
    pub date_of_adopted: Option<String>,
}

impl Info {
    /// The first date the row carries, `yyyy-mm-dd`.
    pub fn any_date(&self) -> Option<&str> {
        [
            &self.date,
            &self.date_of_enactment,
            &self.date_of_issuance,
            &self.date_of_adopted,
        ]
        .into_iter()
        .find_map(|d| d.as_deref().filter(|s| s.len() >= 10))
    }
}

/// One `OmniCorpus` row.
pub struct Row {
    /// Raw text.
    pub text: String,
    /// Parsed `citation_information`.
    pub info: Info,
    /// `E-Library` page.
    pub url: String,
    /// `E-Library` shelf.
    pub label: String,
}

/// Reads every row, calling `keep` on its parsed info first so unwanted rows' text is dropped.
pub fn rows(path: &Path, mut keep: impl FnMut(&Info, &str) -> bool) -> Result<Vec<Row>> {
    let mut out = Vec::new();
    for_each_row(path, |row| {
        let (mut text, mut info, mut url, mut label) =
            (String::new(), None, String::new(), String::new());
        for (name, field) in row.get_column_iter() {
            let value = field_text(field).unwrap_or_default();
            match name.as_str() {
                omni::TEXT => text = value,
                omni::INFO => info = Some(serde_json::from_str::<Info>(&value)?),
                omni::URL => url = value,
                omni::LABEL => label = value,
                _ => {}
            }
        }
        let info = info.ok_or_else(|| anyhow!("{}: row without {}", path.display(), omni::INFO))?;
        if keep(&info, &label) {
            out.push(Row {
                text,
                info,
                url,
                label,
            });
        }
        Ok(())
    })?;
    Ok(out)
}
