//! Maps a model citation to the stored [`Citation`]. Ported from
//! `backend/src/domain/chats/map-citation/v1.ts`: the end page becomes inclusive, a part's pages
//! map back to the original document, the title is capped at 200 and the quote at 4000 UTF-16
//! units. Desktop adds `kind`, `sourceUrl` for statutes, and `\r\n` → `\n` in the quote (spike S4).

use crate::domain::model_request::{AttachedDoc, CitationSource};
use crate::limits::{MAX_DOC_TITLE, MAX_QUOTE};
use crate::model::{Citation, CitationKind};
use crate::util::text::truncate_utf16;

/// Title used when neither the library nor the model names the document.
pub const FALLBACK_TITLE: &str = "Document";

/// Numbers a citation (`n` is 1-based) and ties it to the attached doc it points at.
#[must_use]
pub fn map_citation(source: &CitationSource, docs: &[AttachedDoc], n: u32) -> Citation {
    match source {
        CitationSource::Page {
            document_index,
            document_title,
            start_page,
            end_page_exclusive,
            cited_text,
        } => {
            let doc = docs.get(*document_index);
            let offset = doc.map_or(0, |d| d.page_offset);
            Citation {
                page: Some(start_page + offset),
                end_page: Some(end_page_exclusive.saturating_sub(1) + offset),
                ..doc_citation(n, doc, document_title.as_deref(), cited_text)
            }
        }
        CitationSource::SearchResult {
            source,
            title,
            cited_text,
        } => law_citation(n, source, title, cited_text),
        CitationSource::Other {
            document_index,
            document_title,
            cited_text,
        } => {
            let doc = document_index.and_then(|i| docs.get(i));
            doc_citation(n, doc, document_title.as_deref(), cited_text)
        }
    }
}

fn law_citation(n: u32, source: &str, title: &str, cited_text: &str) -> Citation {
    let title = if title.is_empty() {
        FALLBACK_TITLE
    } else {
        title
    };
    Citation {
        n,
        doc_id: None,
        doc_title: truncate_utf16(title, MAX_DOC_TITLE).to_owned(),
        page: None,
        end_page: None,
        quote: quote(cited_text),
        kind: CitationKind::Law,
        source_url: Some(source.to_owned()),
    }
}

fn doc_citation(n: u32, doc: Option<&AttachedDoc>, reported: Option<&str>, text: &str) -> Citation {
    let title = doc
        .map(|d| d.library_title.as_str())
        .or(reported)
        .unwrap_or(FALLBACK_TITLE);
    Citation {
        n,
        doc_id: doc.map(|d| d.doc_id.clone()),
        doc_title: truncate_utf16(title, MAX_DOC_TITLE).to_owned(),
        page: None,
        end_page: None,
        quote: quote(text),
        kind: CitationKind::Doc,
        source_url: None,
    }
}

fn quote(cited_text: &str) -> String {
    let normalised = cited_text.replace("\r\n", "\n").replace('\r', "\n");
    truncate_utf16(&normalised, MAX_QUOTE).to_owned()
}
