//! Attachment ids in a chat's `docIds`: `<docId>` (the whole doc) or `<docId>:p<n>` (one part,
//! n 1..999), and what they point at. A whole doc is attachable only when ready and unsplit; a
//! part only when the doc is ready and has it. Ported from `domain/docs/attachment-id` and
//! `attachment-target`.

use crate::model::{Doc, DocPart, DocStatus};
use crate::util::text::is_hex_id;

/// A parsed attachment id. `part` is `None` for the whole document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentRef {
    /// 16-hex document id.
    pub doc_id: String,
    /// Part number 1..999, or `None`.
    pub part: Option<u32>,
}

/// The doc as one model document, or one of its parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentTarget<'a> {
    /// The whole document.
    Whole(&'a Doc),
    /// One part of it.
    Part(&'a Doc, &'a DocPart),
}

/// Parses an attachment id. `None` when malformed or the part is 0.
#[must_use]
pub fn parse_attachment_id(id: &str) -> Option<AttachmentRef> {
    let (doc_id, rest) = id.split_at_checked(16)?;
    if !is_hex_id(doc_id) {
        return None;
    }
    if rest.is_empty() {
        return Some(AttachmentRef {
            doc_id: doc_id.to_owned(),
            part: None,
        });
    }
    let digits = rest.strip_prefix(":p")?;
    if digits.is_empty() || digits.len() > 3 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let part = digits.parse::<u32>().ok().filter(|n| *n > 0)?;
    Some(AttachmentRef {
        doc_id: doc_id.to_owned(),
        part: Some(part),
    })
}

/// Whether an attachment id points at `doc_id` (whole or any part).
#[must_use]
pub fn is_attachment_of(id: &str, doc_id: &str) -> bool {
    parse_attachment_id(id).is_some_and(|r| r.doc_id == doc_id)
}

/// Resolves a parsed attachment id against its doc. `None` when the doc is missing, not ready,
/// attached whole although it has parts, or the part does not exist.
#[must_use]
pub fn attachment_target<'a>(
    attachment: &AttachmentRef,
    doc: Option<&'a Doc>,
) -> Option<AttachmentTarget<'a>> {
    let doc = doc.filter(|d| d.status == DocStatus::Ready && d.id == attachment.doc_id)?;
    match attachment.part {
        None => doc.parts.is_empty().then_some(AttachmentTarget::Whole(doc)),
        Some(n) => doc
            .parts
            .iter()
            .find(|p| p.n == n)
            .map(|p| AttachmentTarget::Part(doc, p)),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_attachment_id;
    use super::{AttachmentRef, AttachmentTarget, attachment_target, is_attachment_of};
    use crate::model::{Doc, DocPart, DocStatus};

    const ID: &str = "0123456789abcdef";

    fn doc(status: DocStatus, parts: Vec<DocPart>) -> Doc {
        Doc {
            id: ID.into(),
            title: "T".into(),
            size: 10,
            status,
            created_at_ms: 0,
            page_count: None,
            parts,
            file_id: None,
            processing_attempts: 0,
        }
    }

    fn part(n: u32) -> DocPart {
        DocPart {
            n,
            start_page: 1,
            end_page: 100,
            size: 5,
            file_id: None,
        }
    }

    fn whole() -> AttachmentRef {
        AttachmentRef {
            doc_id: ID.into(),
            part: None,
        }
    }

    #[test]
    fn parses_whole_and_part_ids_and_rejects_malformed_ones() {
        assert_eq!(parse_attachment_id(ID), Some(whole()));
        let p12 = parse_attachment_id(&format!("{ID}:p12")).unwrap();
        assert_eq!(p12.part, Some(12));
        for bad in ["0123456789ABCDEF", "0123456789abcde", "0123456789abcdef:p0"] {
            assert_eq!(parse_attachment_id(bad), None, "{bad}");
        }
        for bad in [
            "0123456789abcdef:p1000",
            "0123456789abcdef:x1",
            "0123456789abcdef:p",
        ] {
            assert_eq!(parse_attachment_id(bad), None, "{bad}");
        }
    }

    #[test]
    fn matches_an_id_and_its_parts_to_the_doc() {
        assert!(is_attachment_of(ID, ID));
        assert!(is_attachment_of(&format!("{ID}:p3"), ID));
        assert!(!is_attachment_of("fedcba9876543210", ID));
    }

    #[test]
    fn attaches_a_ready_unsplit_doc_whole() {
        let d = doc(DocStatus::Ready, vec![]);

        assert_eq!(
            attachment_target(&whole(), Some(&d)),
            Some(AttachmentTarget::Whole(&d))
        );
    }

    #[test]
    fn refuses_a_missing_or_not_ready_doc() {
        let pending = doc(DocStatus::Pending, vec![]);
        let processing = doc(DocStatus::Processing, vec![]);

        assert_eq!(attachment_target(&whole(), None), None);
        assert_eq!(attachment_target(&whole(), Some(&pending)), None);
        assert_eq!(attachment_target(&whole(), Some(&processing)), None);
    }

    #[test]
    fn refuses_a_split_doc_attached_whole_but_attaches_an_existing_part() {
        let split = doc(DocStatus::Ready, vec![part(1), part(2)]);
        let p2 = AttachmentRef {
            doc_id: ID.into(),
            part: Some(2),
        };
        let p3 = AttachmentRef {
            doc_id: ID.into(),
            part: Some(3),
        };

        assert_eq!(attachment_target(&whole(), Some(&split)), None);
        assert_eq!(
            attachment_target(&p2, Some(&split)),
            Some(AttachmentTarget::Part(&split, &split.parts[1]))
        );
        assert_eq!(attachment_target(&p3, Some(&split)), None);
    }

    #[test]
    fn refuses_a_doc_whose_id_differs() {
        let d = doc(DocStatus::Ready, vec![]);
        let other = AttachmentRef {
            doc_id: "fedcba9876543210".into(),
            part: None,
        };

        assert_eq!(attachment_target(&other, Some(&d)), None);
    }
}
