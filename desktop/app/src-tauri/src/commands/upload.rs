//! `upload_chunk`: a raw-body invoke (decision D-8). The body is the chunk's bytes; headers
//! `x-doc-id` and `x-offset` say where they go.

use super::{CommandResult, Shared, blocking};
use crate::composition::Composition;
use kristoff_contracts::envelopes::UploadChunkResponse;
use kristoff_contracts::ipc::{HEADER_DOC_ID, HEADER_OFFSET};
use kristoff_contracts::requests;
use kristoff_core::error::{AppError, ErrorCode};
use kristoff_core::usecases::upload::{self, Chunk};
use tauri::State;
use tauri::ipc::{InvokeBody, Request};

/// What arrived with the invoke, before any check.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawChunk {
    /// `x-doc-id`, if sent.
    pub doc_id: Option<String>,
    /// `x-offset`, if sent.
    pub offset: Option<String>,
    /// The raw body; `None` when the web sent JSON instead of bytes.
    pub body: Option<Vec<u8>>,
}

fn missing(what: &str) -> AppError {
    AppError::new(
        ErrorCode::ValidationFailed,
        format!("{what} is missing or invalid"),
    )
}

/// Writes one chunk.
///
/// # Errors
/// `VALIDATION_FAILED` for a missing or bad header or a JSON body;
/// `DOC_NOT_FOUND`; `UPLOAD_INVALID` from the chunk rules.
pub fn chunk(app: &Composition, raw: &RawChunk) -> CommandResult<UploadChunkResponse> {
    let doc_id = requests::id(
        raw.doc_id
            .as_deref()
            .ok_or_else(|| missing(HEADER_DOC_ID))?,
    )?;
    let offset = raw.offset.as_deref().and_then(|o| o.parse::<u64>().ok());
    let offset = offset.ok_or_else(|| missing(HEADER_OFFSET))?;
    let bytes = raw.body.as_deref().ok_or_else(|| missing("raw body"))?;
    let written = upload::upload_chunk(&app.upload(), doc_id, Chunk { offset, bytes })?;
    Ok(UploadChunkResponse { written })
}

fn header(request: &Request<'_>, name: &str) -> Option<String> {
    request
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// `upload_chunk` (raw body; headers `x-doc-id`, `x-offset`).
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn upload_chunk(
    app: State<'_, Shared>,
    request: Request<'_>,
) -> CommandResult<UploadChunkResponse> {
    let raw = RawChunk {
        doc_id: header(&request, HEADER_DOC_ID),
        offset: header(&request, HEADER_OFFSET),
        body: match request.body() {
            InvokeBody::Raw(bytes) => Some(bytes.clone()),
            InvokeBody::Json(_) => None,
        },
    };
    blocking(&app, move |c| chunk(c, &raw)).await
}
