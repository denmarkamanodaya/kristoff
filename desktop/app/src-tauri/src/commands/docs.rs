//! Library commands: `list_docs`, `create_doc`, `confirm_doc`, `rename_doc`, `get_doc_link` and
//! `delete_doc`. Uploads: `upload.rs`.

use super::{CommandResult, Shared, blocking, unlocked};
use crate::composition::Composition;
use kristoff_contracts::envelopes::{
    DeletedResponse, DocResponse, DocUploadResponse, DocsPageResponse, LinkResponse,
};
use kristoff_contracts::requests;
use kristoff_contracts::shapes::{DocDto, UploadHeadersDto};
use kristoff_core::limits::PDF_CONTENT_TYPE;
use kristoff_core::usecases::{docs, upload};
use kristoff_core::util::time::iso_instant;
use tauri::State;

/// One page of the library.
///
/// # Errors
/// `UNAUTHENTICATED`; `VALIDATION_FAILED` for a bad cursor.
pub fn list(app: &Composition, cursor: Option<&str>) -> CommandResult<DocsPageResponse> {
    unlocked(app)?;
    let page = docs::list_docs(&app.library(), requests::cursor(cursor)?)?;
    let items = page.items.iter().map(DocDto::from).collect();
    Ok(DocsPageResponse {
        items,
        next_cursor: page.next_cursor,
    })
}

/// Records a pending doc and returns where its bytes go.
///
/// # Errors
/// `UNAUTHENTICATED`; `VALIDATION_FAILED` for a bad name or size.
pub fn create(app: &Composition, file_name: &str, size: u64) -> CommandResult<DocUploadResponse> {
    unlocked(app)?;
    let input = requests::create_doc(file_name, size)?;
    let created = docs::create_doc(&app.library(), &input)?;
    Ok(DocUploadResponse {
        doc: DocDto::from(&created.doc),
        upload_url: created.upload_url,
        upload_headers: UploadHeadersDto {
            content_type: PDF_CONTENT_TYPE.to_owned(),
        },
    })
}

/// Confirms an uploaded doc.
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`, `DOC_NOT_FOUND`, `UPLOAD_INVALID`.
pub fn confirm(app: &Composition, doc_id: &str) -> CommandResult<DocResponse> {
    unlocked(app)?;
    let doc = upload::confirm_doc(&app.upload(), requests::id(doc_id)?)?;
    Ok(DocResponse {
        doc: DocDto::from(&doc),
    })
}

/// Renames a doc.
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`, `DOC_NOT_FOUND`.
pub fn rename(app: &Composition, doc_id: &str, title: &str) -> CommandResult<DocResponse> {
    unlocked(app)?;
    let (doc_id, title) = (requests::id(doc_id)?, requests::doc_title(title)?);
    let doc = docs::rename_doc(&app.library(), doc_id, title)?;
    Ok(DocResponse {
        doc: DocDto::from(&doc),
    })
}

/// The `kristoff://doc/...` link of a ready doc or part.
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`, `DOC_NOT_FOUND`.
pub fn link(app: &Composition, doc_id: &str, part: Option<u32>) -> CommandResult<LinkResponse> {
    unlocked(app)?;
    let (doc_id, part) = (requests::id(doc_id)?, requests::link_part(part)?);
    let link = docs::get_doc_link(&app.library(), doc_id, part)?;
    Ok(LinkResponse {
        url: link.url,
        expires_at: iso_instant(link.expires_at_ms),
    })
}

/// Deletes a doc everywhere: chats, disk, then the record.
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`, `DOC_NOT_FOUND`.
pub fn delete(app: &Composition, doc_id: &str) -> CommandResult<DeletedResponse> {
    unlocked(app)?;
    docs::delete_doc(&app.library(), requests::id(doc_id)?)?;
    Ok(DeletedResponse { deleted: true })
}

/// `list_docs {cursor}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn list_docs(
    app: State<'_, Shared>,
    cursor: Option<String>,
) -> CommandResult<DocsPageResponse> {
    blocking(&app, move |c| list(c, cursor.as_deref())).await
}

/// `create_doc {fileName, size}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn create_doc(
    app: State<'_, Shared>,
    file_name: String,
    size: u64,
) -> CommandResult<DocUploadResponse> {
    blocking(&app, move |c| create(c, &file_name, size)).await
}

/// `confirm_doc {docId}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn confirm_doc(app: State<'_, Shared>, doc_id: String) -> CommandResult<DocResponse> {
    blocking(&app, move |c| confirm(c, &doc_id)).await
}

/// `rename_doc {docId, title}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn rename_doc(
    app: State<'_, Shared>,
    doc_id: String,
    title: String,
) -> CommandResult<DocResponse> {
    blocking(&app, move |c| rename(c, &doc_id, &title)).await
}

/// `get_doc_link {docId, part}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn get_doc_link(
    app: State<'_, Shared>,
    doc_id: String,
    part: Option<u32>,
) -> CommandResult<LinkResponse> {
    blocking(&app, move |c| link(c, &doc_id, part)).await
}

/// `delete_doc {docId}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn delete_doc(app: State<'_, Shared>, doc_id: String) -> CommandResult<DeletedResponse> {
    blocking(&app, move |c| delete(c, &doc_id)).await
}
