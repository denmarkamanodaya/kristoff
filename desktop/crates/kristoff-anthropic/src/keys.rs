//! `KeyVerifier` on `GET /v1/models?limit=1` (Phase 8, docs/desktop/phases/phase-8-lexph-byok.md
//! §1): a free call that succeeds only with a working key. The candidate key is sent on this one
//! request only; it is never stored here or logged.

use crate::bridge::AnthropicRuntime;
use crate::error::ErrorCode as Code;
use crate::http::{self, with_key};
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::ports::KeyVerifier;
use reqwest::{Client, Method};
use std::sync::Arc;

/// The check path.
const MODELS_PATH: &str = "/v1/models?limit=1";

/// HTTP 403: the key exists but may not call the API (treated as a rejected key).
const FORBIDDEN: u16 = 403;

/// Checks keys with one models-list call on the shared client and runtime.
pub struct AnthropicKeyVerifier {
    rt: Arc<AnthropicRuntime>,
    client: Client,
    base_url: String,
}

impl AnthropicKeyVerifier {
    /// A verifier calling `base_url` (no trailing slash; production [`crate::API_BASE`]).
    #[must_use]
    pub fn new(rt: Arc<AnthropicRuntime>, client: Client, base_url: &str) -> Self {
        Self {
            rt,
            client,
            base_url: base_url.to_owned(),
        }
    }
}

impl KeyVerifier for AnthropicKeyVerifier {
    fn verify(&self, key: &str) -> AppResult<()> {
        let url = format!("{}{MODELS_PATH}", self.base_url);
        let make = || with_key(self.client.request(Method::GET, &url), key);
        let sent = self.rt.block_on(http::send(make, None));
        match sent {
            Ok(_) => Ok(()),
            Err(e) if e.code == Code::KeyRejected || e.status == Some(FORBIDDEN) => {
                Err(AppError::new(
                    ErrorCode::KeyInvalid,
                    "Anthropic rejected the Claude API key",
                ))
            }
            Err(e) => Err(AppError::new(
                ErrorCode::UpstreamUnavailable,
                format!("the key check failed: {e}"),
            )),
        }
    }
}
