//! `open_doc` (decision D-12: `WKWebView` blocks `window.open`, so the shell opens a viewer window at
//! `kristoff://doc/...#page=N`) and `open_external` (allow-listed https links in the default
//! browser). Both are gated.

use super::{CommandResult, Shared, blocking, tag_failure, unlocked};
use crate::composition::Composition;
use kristoff_contracts::config::EXTERNAL_HOSTS;
use kristoff_contracts::error::ErrorDto;
use kristoff_contracts::requests;
use kristoff_core::domain::external::is_allowed_external;
use kristoff_core::error::{AppError, ErrorCode};
use kristoff_core::usecases::viewer::{self, DocWindow};
use tauri::{AppHandle, Manager, State, Url, WebviewUrl, WebviewWindowBuilder};

/// Longest URL `open_external` accepts.
const MAX_URL: usize = 2048;

fn internal(e: &impl std::fmt::Display) -> ErrorDto {
    AppError::new(ErrorCode::Internal, e.to_string()).into()
}

/// The viewer window of a ready doc or part at a page (relative to the part).
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`, `DOC_NOT_FOUND`.
pub fn doc_window(
    app: &Composition,
    doc_id: &str,
    part: Option<u32>,
    page: Option<u32>,
) -> CommandResult<DocWindow> {
    unlocked(app)?;
    let (doc_id, part) = (requests::id(doc_id)?, requests::link_part(part)?);
    Ok(viewer::doc_window(app.viewer().0, doc_id, part, page)?)
}

/// Checks a link for `open_external`: https and an allow-listed host.
///
/// # Errors
/// `UNAUTHENTICATED`; `VALIDATION_FAILED` for anything else.
pub fn external_url<'a>(app: &Composition, url: &'a str) -> CommandResult<&'a str> {
    unlocked(app)?;
    if url.len() > MAX_URL || !is_allowed_external(url, &EXTERNAL_HOSTS) {
        return Err(
            AppError::new(ErrorCode::ValidationFailed, "this link cannot be opened").into(),
        );
    }
    Ok(url)
}

/// Focuses the viewer with that label (moving it to the new page), or opens a new one.
fn show(handle: &AppHandle, window: &DocWindow) -> CommandResult<()> {
    let url: Url = window.url.parse().map_err(|e| internal(&e))?;
    if let Some(open) = handle.get_webview_window(&window.label) {
        open.navigate(url).map_err(|e| internal(&e))?;
        return open.set_focus().map_err(|e| internal(&e));
    }
    WebviewWindowBuilder::new(handle, &window.label, WebviewUrl::External(url))
        .title(&window.title)
        .inner_size(900.0, 1000.0)
        .build()
        .map(|_| ())
        .map_err(|e| internal(&e))
}

/// Opens `url` in the default browser with macOS `open` (no shell; the URL is one argument).
fn launch(url: &str) -> CommandResult<()> {
    let status = std::process::Command::new("/usr/bin/open")
        .arg(url)
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(internal(&format!("open exited with {s}"))),
        Err(e) => Err(internal(&e)),
    }
}

/// `open_doc {docId, part, page}`.
///
/// # Errors
/// Those of [`doc_window`], or `INTERNAL` when the window cannot be shown.
// State, the app handle and the three IPC arguments {docId, part, page} are the wire contract.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn open_doc(
    handle: AppHandle,
    app: State<'_, Shared>,
    doc_id: String,
    part: Option<u32>,
    page: Option<u32>,
) -> CommandResult<()> {
    let window = blocking(&app, move |c| doc_window(c, &doc_id, part, page)).await?;
    tag_failure(&app, show(&handle, &window))
}

/// `open_external {url}`.
///
/// # Errors
/// Those of [`external_url`], or `INTERNAL` when `open` fails.
#[tauri::command]
pub async fn open_external(app: State<'_, Shared>, url: String) -> CommandResult<()> {
    blocking(&app, move |c| launch(external_url(c, &url)?)).await
}
