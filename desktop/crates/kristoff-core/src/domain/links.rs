//! The desktop's two URL schemes: where the web uploads bytes (`kristoff-upload://<docId>`, handed
//! to `upload_chunk`) and how it opens a PDF (`kristoff://doc/<docId>[/part-<n>]`, decision D-9).

/// Prefix of the upload URL `create_doc` returns.
pub const UPLOAD_URL_PREFIX: &str = "kristoff-upload://";
/// Prefix of a document link.
pub const DOC_LINK_PREFIX: &str = "kristoff://doc/";

/// The upload URL of a pending doc.
#[must_use]
pub fn upload_url(doc_id: &str) -> String {
    format!("{UPLOAD_URL_PREFIX}{doc_id}")
}

/// The link of a doc, or of one of its parts.
#[must_use]
pub fn doc_link(doc_id: &str, part: Option<u32>) -> String {
    match part {
        None => format!("{DOC_LINK_PREFIX}{doc_id}"),
        Some(n) => format!("{DOC_LINK_PREFIX}{doc_id}/part-{n}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{doc_link, upload_url};

    #[test]
    fn builds_the_upload_url_and_doc_links() {
        assert_eq!(
            upload_url("0123456789abcdef"),
            "kristoff-upload://0123456789abcdef"
        );
        assert_eq!(
            doc_link("0123456789abcdef", None),
            "kristoff://doc/0123456789abcdef"
        );
        assert_eq!(
            doc_link("0123456789abcdef", Some(3)),
            "kristoff://doc/0123456789abcdef/part-3"
        );
    }
}
