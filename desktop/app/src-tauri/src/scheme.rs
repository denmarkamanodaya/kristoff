//! The `kristoff://` scheme (decision D-12): serves a ready doc's or part's PDF bytes to the viewer
//! window. 403 while the app is locked, 404 for any other path or a doc that is missing, not
//! ready or lacks the part. The `#page=N` fragment never reaches here; the PDF viewer reads it.

use crate::commands::Shared;
use crate::composition::Composition;
use kristoff_core::domain::doc_path::parse_doc_path;
use kristoff_core::error::ErrorCode;
use kristoff_core::limits::PDF_CONTENT_TYPE;
use kristoff_core::usecases::viewer::read_doc_file;
use std::sync::Arc;
use tauri::http::{Request, Response, StatusCode, Uri, header};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

/// What the handler answers: a status and, for 200, the PDF bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// HTTP status.
    pub status: StatusCode,
    /// The PDF bytes (empty unless 200).
    pub body: Vec<u8>,
}

fn empty(status: StatusCode) -> Reply {
    Reply {
        status,
        body: Vec::new(),
    }
}

/// The scheme path of a URI: `kristoff://doc/<id>/part-2` has host `doc` and path `/<id>/part-2`,
/// which join to `doc/<id>/part-2`.
#[must_use]
pub fn scheme_path(uri: &Uri) -> String {
    format!("{}{}", uri.host().unwrap_or_default(), uri.path())
}

/// Answers one scheme request for `path` (see [`scheme_path`]).
#[must_use]
pub fn reply(app: &Composition, path: &str) -> Reply {
    if app.gate.check(app.now_ms()).is_err() {
        return empty(StatusCode::FORBIDDEN);
    }
    let Some(target) = parse_doc_path(path) else {
        return empty(StatusCode::NOT_FOUND);
    };
    let (docs, files) = app.viewer();
    match read_doc_file(docs, files, &target.doc_id, target.part) {
        Ok(body) => Reply {
            status: StatusCode::OK,
            body,
        },
        Err(e) if e.code == ErrorCode::DocNotFound => empty(StatusCode::NOT_FOUND),
        Err(_) => empty(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// The HTTP response for a reply (`application/pdf` on 200).
#[must_use]
pub fn response(reply: Reply) -> Response<Vec<u8>> {
    let mut builder = Response::builder().status(reply.status);
    if reply.status == StatusCode::OK {
        builder = builder.header(header::CONTENT_TYPE, PDF_CONTENT_TYPE);
    }
    builder
        .body(reply.body)
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

/// The registered handler: answers on its own thread, since reading a PDF of up to 200 MB is
/// blocking IO that must never run on the webview's thread.
// Tauri's handler signature hands the context and request by value.
#[allow(clippy::needless_pass_by_value)]
pub fn handle<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = ctx
        .app_handle()
        .try_state::<Shared>()
        .map(|s| Arc::clone(s.inner()));
    let path = scheme_path(request.uri());
    std::thread::spawn(move || {
        let answer = app.map_or_else(
            || empty(StatusCode::SERVICE_UNAVAILABLE),
            |a| reply(&a, &path),
        );
        responder.respond(response(answer));
    });
}
