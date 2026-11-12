//! The `kristoff://doc/...` viewer address (decision D-12): parsing the path the scheme handler
//! receives, the window label of a doc or part, and the URL a viewer window opens.

use crate::domain::links::doc_link;
use crate::limits::MAX_PARTS;
use crate::util::text::is_hex_id;

/// A doc or one of its parts, as named by a viewer path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocRef {
    /// 16-hex doc id.
    pub doc_id: String,
    /// Part number 1..200, or `None` for the whole PDF.
    pub part: Option<u32>,
}

/// Parses a scheme path: exactly `doc/<16hex>` or `doc/<16hex>/part-<1..200>` (no leading or
/// trailing slash, no leading zeros, nothing else). `None` for anything else.
#[must_use]
pub fn parse_doc_path(path: &str) -> Option<DocRef> {
    let rest = path.strip_prefix("doc/")?;
    let (doc_id, tail) = rest.split_at_checked(16)?;
    if !is_hex_id(doc_id) {
        return None;
    }
    if tail.is_empty() {
        return Some(DocRef {
            doc_id: doc_id.to_owned(),
            part: None,
        });
    }
    let digits = tail.strip_prefix("/part-")?;
    let canonical = !digits.is_empty() && !digits.starts_with('0') && digits.len() <= 3;
    if !canonical || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n = digits.parse::<u32>().ok()?;
    let in_range = usize::try_from(n).is_ok_and(|n| (1..=MAX_PARTS).contains(&n));
    in_range.then(|| DocRef {
        doc_id: doc_id.to_owned(),
        part: Some(n),
    })
}

/// The window label of a doc or part viewer, unique per doc and part.
#[must_use]
pub fn viewer_label(doc_id: &str, part: Option<u32>) -> String {
    match part {
        None => format!("doc-{doc_id}"),
        Some(n) => format!("doc-{doc_id}-part-{n}"),
    }
}

/// The viewer URL, with `#page=<N>` when a page is given (1-based within the file or part).
#[must_use]
pub fn viewer_url(doc_id: &str, part: Option<u32>, page: Option<u32>) -> String {
    let link = doc_link(doc_id, part);
    match page {
        None => link,
        Some(n) => format!("{link}#page={n}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{DocRef, parse_doc_path, viewer_label, viewer_url};

    const ID: &str = "0123456789abcdef";

    #[test]
    fn parses_a_whole_doc_and_a_part() {
        assert_eq!(
            parse_doc_path(&format!("doc/{ID}")),
            Some(DocRef {
                doc_id: ID.into(),
                part: None
            })
        );
        let part = parse_doc_path(&format!("doc/{ID}/part-200")).unwrap();
        assert_eq!(part.part, Some(200));
    }

    #[test]
    fn rejects_every_other_path() {
        let bad = [
            String::new(),
            ID.to_owned(),
            format!("/doc/{ID}"),
            format!("doc/{ID}/"),
            format!("doc/{}", ID.to_uppercase()),
            format!("doc/{ID}x"),
            format!("doc/{ID}/part-0"),
            format!("doc/{ID}/part-01"),
            format!("doc/{ID}/part-201"),
            format!("doc/{ID}/part-"),
            format!("doc/{ID}/part-1/x"),
            format!("doc/{ID}/../../etc/passwd"),
            format!("docs/{ID}"),
        ];
        for path in bad {
            assert_eq!(parse_doc_path(&path), None, "{path}");
        }
    }

    #[test]
    fn labels_and_urls_name_the_doc_part_and_page() {
        assert_eq!(viewer_label(ID, None), format!("doc-{ID}"));
        assert_eq!(viewer_label(ID, Some(2)), format!("doc-{ID}-part-2"));
        assert_eq!(viewer_url(ID, None, None), format!("kristoff://doc/{ID}"));
        assert_eq!(
            viewer_url(ID, Some(2), Some(7)),
            format!("kristoff://doc/{ID}/part-2#page=7")
        );
    }
}
