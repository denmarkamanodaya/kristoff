//! Request checks with the bounds of `backend/src/contracts/requests.ts` and `domain/limits.ts`.
//! Every failure is `VALIDATION_FAILED`. Lengths are UTF-16 units, as in the TypeScript contract.

use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::limits::{
    MAX_CHAT_TITLE, MAX_CURSOR, MAX_DOC_IDS_CEILING, MAX_DOC_TITLE, MAX_FILE_NAME,
    MAX_PROJECT_NAME, MAX_QUESTION, MAX_UPLOAD_BYTES,
};
use kristoff_core::ports::{ChatChanges, ProjectChange};
use kristoff_core::usecases::docs::DocUploadInput;
use kristoff_core::util::text::{is_hex_id, utf16_len};
use serde::{Deserialize, Deserializer};

/// Longest model id in a question.
const MAX_MODEL_ID: usize = 64;
/// Highest part number a link may name.
const MAX_LINK_PART: u32 = 999;

fn invalid(what: &str) -> AppError {
    AppError::new(ErrorCode::ValidationFailed, format!("{what} is invalid"))
}

fn bounded<'a>(text: &'a str, max: usize, what: &str) -> AppResult<&'a str> {
    let len = utf16_len(text);
    if len == 0 || len > max {
        Err(invalid(what))
    } else {
        Ok(text)
    }
}

/// A 16-hex id.
///
/// # Errors
/// `VALIDATION_FAILED` otherwise.
pub fn id(text: &str) -> AppResult<&str> {
    if is_hex_id(text) {
        Ok(text)
    } else {
        Err(invalid("id"))
    }
}

/// A six-digit code.
///
/// # Errors
/// `VALIDATION_FAILED` otherwise.
pub fn otp_code(code: &str) -> AppResult<&str> {
    let six_digits = code.len() == 6 && code.bytes().all(|b| b.is_ascii_digit());
    if six_digits {
        Ok(code)
    } else {
        Err(invalid("code"))
    }
}

/// An optional opaque cursor, 1..512.
///
/// # Errors
/// `VALIDATION_FAILED` for an empty or long cursor.
pub fn cursor(cursor: Option<&str>) -> AppResult<Option<&str>> {
    cursor.map(|c| bounded(c, MAX_CURSOR, "cursor")).transpose()
}

/// `create_doc {fileName, size}`: name 1..255, size 1..200 MiB.
///
/// # Errors
/// `VALIDATION_FAILED` outside those bounds.
pub fn create_doc(file_name: &str, size: u64) -> AppResult<DocUploadInput> {
    bounded(file_name, MAX_FILE_NAME, "fileName")?;
    if size == 0 || size > MAX_UPLOAD_BYTES {
        return Err(invalid("size"));
    }
    Ok(DocUploadInput {
        file_name: file_name.to_owned(),
        size,
    })
}

/// A doc title, 1..200 (trimmed by the use case).
///
/// # Errors
/// `VALIDATION_FAILED` outside those bounds.
pub fn doc_title(title: &str) -> AppResult<&str> {
    bounded(title, MAX_DOC_TITLE, "title")
}

/// A project name, 1..80 (trimmed by the use case).
///
/// # Errors
/// `VALIDATION_FAILED` outside those bounds.
pub fn project_name(name: &str) -> AppResult<&str> {
    bounded(name, MAX_PROJECT_NAME, "name")
}

/// An optional part number of a link, 1..999.
///
/// # Errors
/// `VALIDATION_FAILED` outside those bounds.
pub fn link_part(part: Option<u32>) -> AppResult<Option<u32>> {
    match part {
        Some(n) if n == 0 || n > MAX_LINK_PART => Err(invalid("part")),
        other => Ok(other),
    }
}

/// Whether `text` has the attachment-id shape `<16 hex>` or `<16 hex>:p<1-3 digits>`.
fn is_attachment_shape(text: &str) -> bool {
    let (doc, rest) = text.split_at_checked(16).unwrap_or(("", "x"));
    let part_ok = rest.is_empty()
        || rest
            .strip_prefix(":p")
            .is_some_and(|d| (1..=3).contains(&d.len()) && d.bytes().all(|b| b.is_ascii_digit()));
    is_hex_id(doc) && part_ok
}

/// Attachment ids: at most 20, each `<docId>` or `<docId>:p<n>`.
///
/// # Errors
/// `VALIDATION_FAILED` for too many or a malformed id.
pub fn doc_ids(ids: &[String]) -> AppResult<&[String]> {
    let ok = ids.len() <= MAX_DOC_IDS_CEILING && ids.iter().all(|i| is_attachment_shape(i));
    if ok { Ok(ids) } else { Err(invalid("docIds")) }
}

/// `ask {text, model}`: text 1..8000, model 1..64 when sent.
///
/// # Errors
/// `VALIDATION_FAILED` outside those bounds.
pub fn ask<'a>(text: &'a str, model: Option<&'a str>) -> AppResult<(&'a str, Option<&'a str>)> {
    bounded(text, MAX_QUESTION, "text")?;
    let model = model
        .map(|m| bounded(m, MAX_MODEL_ID, "model"))
        .transpose()?;
    Ok((text, model))
}

/// `projectId`: null ungroups, a string moves; absent stays `None` (with `#[serde(default)]`).
fn project_change<'de, D: Deserializer<'de>>(d: D) -> Result<Option<ProjectChange>, D::Error> {
    let raw = Option::<String>::deserialize(d)?;
    Ok(Some(
        raw.map_or(ProjectChange::Ungroup, ProjectChange::MoveTo),
    ))
}

/// `update_chat {patch}`: at least one field; `projectId: null` ungroups.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChatPatchRequest {
    /// New title, 1..120.
    pub title: Option<String>,
    /// New attachment ids.
    pub doc_ids: Option<Vec<String>>,
    /// Pin or unpin.
    pub pinned: Option<bool>,
    /// Move into a project, or null to ungroup.
    #[serde(default, deserialize_with = "project_change")]
    pub project_id: Option<ProjectChange>,
    /// Turn statute search on or off (desktop, Phase 4).
    pub statutes: Option<bool>,
}

impl ChatPatchRequest {
    /// Parses a raw `patch` argument (unknown fields and wrong types are `VALIDATION_FAILED`).
    ///
    /// # Errors
    /// `VALIDATION_FAILED` when it does not parse or fails [`Self::check`].
    pub fn parse(raw: serde_json::Value) -> AppResult<ChatChanges> {
        let request: Self = serde_json::from_value(raw).map_err(|_| invalid("patch"))?;
        request.check()
    }

    /// Checks the bounds and that at least one field is present.
    ///
    /// # Errors
    /// `VALIDATION_FAILED` for an empty patch or a field out of bounds.
    pub fn check(self) -> AppResult<ChatChanges> {
        let empty = self.title.is_none()
            && self.doc_ids.is_none()
            && self.pinned.is_none()
            && self.project_id.is_none()
            && self.statutes.is_none();
        if empty {
            return Err(invalid(
                "patch (send at least one of title, docIds, pinned, projectId, statutes)",
            ));
        }
        if let Some(title) = &self.title {
            bounded(title, MAX_CHAT_TITLE, "title")?;
        }
        if let Some(ids) = &self.doc_ids {
            doc_ids(ids)?;
        }
        if let Some(ProjectChange::MoveTo(project)) = &self.project_id {
            id(project)?;
        }
        let Self {
            title,
            doc_ids,
            pinned,
            project_id,
            statutes,
        } = self;
        Ok(ChatChanges {
            title,
            doc_ids,
            pinned,
            project: project_id,
            statutes,
        })
    }
}
