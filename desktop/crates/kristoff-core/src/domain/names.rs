//! Naming rules: a doc title from a file name, and the trimmed non-blank names of docs, chats and
//! projects. Ported from `create-doc-upload` (`titleOf`), `rename-doc` and `project-name`.

use crate::error::{AppError, AppResult, ErrorCode};
use crate::limits::{MAX_DOC_TITLE, MAX_PROJECT_NAME};
use crate::util::text::{truncate_utf16, utf16_len};

/// Title shown for a file whose name is only an extension or blank.
const UNTITLED: &str = "Document";

/// The doc title for an uploaded file name: `.pdf` (any case) dropped, trimmed, cut to 200.
#[must_use]
pub fn title_of(file_name: &str) -> String {
    let cut = file_name.len().saturating_sub(4);
    let is_pdf = file_name
        .get(cut..)
        .is_some_and(|ext| ext.eq_ignore_ascii_case(".pdf"));
    let stem = if is_pdf { &file_name[..cut] } else { file_name };
    let title = truncate_utf16(stem.trim(), MAX_DOC_TITLE);
    if title.is_empty() {
        UNTITLED.to_owned()
    } else {
        title.to_owned()
    }
}

/// A trimmed, non-blank name of at most `max` UTF-16 units.
///
/// # Errors
/// `VALIDATION_FAILED` naming `what` when the trimmed name is blank or too long.
pub fn trimmed_name(raw: &str, max: usize, what: &str) -> AppResult<String> {
    let name = raw.trim();
    if name.is_empty() || utf16_len(name) > max {
        return Err(AppError::new(
            ErrorCode::ValidationFailed,
            format!("{what} must be 1 to {max} characters"),
        ));
    }
    Ok(name.to_owned())
}

/// A project name: trimmed, 1..80.
///
/// # Errors
/// `VALIDATION_FAILED` when blank or longer than 80.
pub fn project_name(raw: &str) -> AppResult<String> {
    trimmed_name(raw, MAX_PROJECT_NAME, "name")
}

#[cfg(test)]
mod tests {
    use super::{project_name, title_of};
    use crate::error::ErrorCode;

    #[test]
    fn titles_a_file_without_its_pdf_extension() {
        assert_eq!(title_of("People v. Cruz.PDF"), "People v. Cruz");
        assert_eq!(title_of("  notes.pdf "), "notes.pdf");
        assert_eq!(title_of(".pdf"), "Document");
        assert_eq!(title_of(&"x".repeat(300)).len(), 200);
    }

    #[test]
    fn trims_project_names_and_refuses_blank_or_long_ones() {
        assert_eq!(project_name("  Torts "), Ok("Torts".to_owned()));
        assert_eq!(
            project_name("   ").unwrap_err().code,
            ErrorCode::ValidationFailed
        );
        assert_eq!(
            project_name(&"x".repeat(81)).unwrap_err().code,
            ErrorCode::ValidationFailed
        );
    }
}
