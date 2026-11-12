//! Builders for domain values used across tests.

use kristoff_core::model::{
    Chat, Citation, CitationKind, Doc, DocPart, DocStatus, Message, Project, Role, Usage,
};

/// A doc with id `id`, status `status`, 1000 bytes, no parts.
#[must_use]
pub fn doc(id: &str, status: DocStatus) -> Doc {
    Doc {
        id: id.to_owned(),
        title: format!("Doc {id}"),
        size: 1000,
        status,
        created_at_ms: 1_794_445_200_000,
        page_count: None,
        parts: vec![],
        file_id: None,
        processing_attempts: 0,
    }
}

/// A ready, uploaded, unsplit doc: file id `file_<id>`.
#[must_use]
pub fn uploaded_doc(id: &str) -> Doc {
    Doc {
        file_id: Some(format!("file_{id}")),
        ..doc(id, DocStatus::Ready)
    }
}

/// Part `n` covering pages `100(n-1)+1 ..= 100n`.
#[must_use]
pub fn part(n: u32) -> DocPart {
    DocPart {
        n,
        start_page: 100 * (n - 1) + 1,
        end_page: 100 * n,
        size: 500,
        file_id: Some(format!("file_part_{n}")),
    }
}

/// A chat with id `id`, updated at `updated_at_ms`, attaching `doc_ids`.
#[must_use]
pub fn chat(id: &str, updated_at_ms: i64, doc_ids: &[&str]) -> Chat {
    Chat {
        id: id.to_owned(),
        title: "New chat".to_owned(),
        doc_ids: doc_ids.iter().map(|d| (*d).to_owned()).collect(),
        pinned: false,
        project_id: None,
        created_at_ms: updated_at_ms,
        updated_at_ms,
    }
}

/// A project named `name`.
#[must_use]
pub fn project(id: &str, name: &str) -> Project {
    Project {
        id: id.to_owned(),
        name: name.to_owned(),
        created_at_ms: 1_794_445_200_000,
        updated_at_ms: 1_794_445_200_000,
    }
}

/// An assistant answer with one doc and one law citation.
#[must_use]
pub fn answer(id: &str, at_ms: i64) -> Message {
    let doc_citation = Citation {
        n: 1,
        doc_id: Some("0123456789abcdef".to_owned()),
        doc_title: "People v. Cruz".to_owned(),
        page: Some(3),
        end_page: Some(4),
        quote: "The accused is acquitted.".to_owned(),
        kind: CitationKind::Doc,
        source_url: None,
    };
    let law_citation = Citation {
        n: 2,
        doc_id: None,
        doc_title: "RA 9262 · Sec. 5".to_owned(),
        page: None,
        end_page: None,
        quote: "Acts of violence against women".to_owned(),
        kind: CitationKind::Law,
        source_url: Some("https://lawphil.net/statutes/repacts/ra2004/ra_9262_2004.html".into()),
    };
    Message {
        id: id.to_owned(),
        role: Role::Assistant,
        text: "Acquitted [1] under RA 9262 [2].".to_owned(),
        citations: vec![doc_citation, law_citation],
        usage: Some(Usage {
            input: 10,
            output: 20,
            cache_read: 3,
            cache_write: 4,
        }),
        model: Some("claude-sonnet-5-5".to_owned()),
        stop_reason: Some("end_turn".to_owned()),
        at_ms,
        duration_ms: Some(1200),
        follow_ups: vec!["What is the ratio?".to_owned()],
    }
}

/// A user question.
#[must_use]
pub fn question(id: &str, at_ms: i64) -> Message {
    Message {
        id: id.to_owned(),
        role: Role::User,
        text: "Was he acquitted?".to_owned(),
        citations: vec![],
        usage: None,
        model: None,
        stop_reason: None,
        at_ms,
        duration_ms: None,
        follow_ups: vec![],
    }
}
