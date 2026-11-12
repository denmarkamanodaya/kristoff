//! `AnthropicFiles`: Files API upload and delete (GA, no beta header: D-11).

use crate::error::{AnthropicError, ErrorCode};
use crate::http::{self, ApiConfig};
use kristoff_core::limits::PDF_CONTENT_TYPE;
use reqwest::multipart::{Form, Part};
use reqwest::{Client, Method};

/// Longest file id accepted back from the API.
const MAX_FILE_ID: usize = 128;

/// Uploads and deletes PDFs in the Files API.
#[derive(Clone)]
pub struct AnthropicFiles {
    client: Client,
    config: ApiConfig,
}

impl AnthropicFiles {
    /// Wraps the shared client (see [`crate::build_client`]).
    #[must_use]
    pub fn new(client: Client, config: ApiConfig) -> Self {
        Self { client, config }
    }

    /// Uploads a PDF under `name` and returns its file id.
    ///
    /// # Errors
    /// HTTP failures as in [`crate::AnthropicModel`]; `UPSTREAM_INVALID_RESPONSE` when the reply
    /// has no valid `id`.
    pub async fn upload(&self, name: &str, bytes: &[u8]) -> Result<String, AnthropicError> {
        // Checked once here, so the per-attempt `mime_str` below cannot fail.
        Part::bytes(Vec::new())
            .mime_str(PDF_CONTENT_TYPE)
            .map_err(|e| AnthropicError::from_reqwest(&e))?;
        let make = || {
            let part = Part::bytes(bytes.to_vec()).file_name(name.to_owned());
            let part = part
                .mime_str(PDF_CONTENT_TYPE)
                .unwrap_or_else(|_| Part::bytes(bytes.to_vec()));
            self.config
                .request(&self.client, Method::POST, "/v1/files")
                .multipart(Form::new().part("file", part))
        };
        let response = http::send(make, None).await?;
        let body = response
            .bytes()
            .await
            .map_err(|e| AnthropicError::from_reqwest(&e))?;
        file_id_of(&body)
    }

    /// Deletes a file.
    ///
    /// # Errors
    /// `UPSTREAM_REJECTED` for an id that is not a valid file id (never sent) or a 4xx (404
    /// included: the caller treats delete as best effort); otherwise as in `upload`.
    pub async fn delete(&self, file_id: &str) -> Result<(), AnthropicError> {
        if !is_file_id(file_id) {
            return Err(AnthropicError::with_type(
                ErrorCode::UpstreamRejected,
                "bad_file_id",
            ));
        }
        let path = format!("/v1/files/{file_id}");
        let make = || self.config.request(&self.client, Method::DELETE, &path);
        http::send(make, None).await.map(drop)
    }
}

/// Reads `id` from an upload reply and checks it is a plain file id.
fn file_id_of(body: &[u8]) -> Result<String, AnthropicError> {
    let invalid = || AnthropicError::with_type(ErrorCode::UpstreamInvalidResponse, "file_id");
    let value: serde_json::Value = serde_json::from_slice(body).map_err(|_| invalid())?;
    let id = value["id"].as_str().ok_or_else(invalid)?;
    if is_file_id(id) {
        Ok(id.to_owned())
    } else {
        Err(invalid())
    }
}

/// A file id is used in a URL path, so only `[A-Za-z0-9_-]{1,128}` is accepted.
fn is_file_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_FILE_ID
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_id_from_an_upload_reply() {
        let body = br#"{"type":"file","id":"file_011ABC","filename":"a.pdf"}"#;
        assert_eq!(file_id_of(body).unwrap(), "file_011ABC");
    }

    #[test]
    fn rejects_an_id_that_could_change_the_url_path() {
        assert!(file_id_of(br#"{"id":"../v1/messages"}"#).is_err());
        assert!(!is_file_id("a/b"));
        assert!(!is_file_id(""));
    }
}
