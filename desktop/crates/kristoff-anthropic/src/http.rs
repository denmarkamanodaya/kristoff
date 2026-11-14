//! The shared HTTP client and the one-retry rule (stack/errors-and-resilience.md "Retries").

use crate::cancel::CancelToken;
use crate::error::{AnthropicError, ErrorCode};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use std::sync::{Arc, PoisonError, RwLock};
use std::time::Duration;

/// API version header value (D-11). The Files API is GA, so no beta header is sent.
pub const ANTHROPIC_VERSION: &str = "2026-10-21";

/// Default API origin.
pub const API_BASE: &str = "https://api.anthropic.com";

/// Connect timeout.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Whole-request timeout (`ANTHROPIC_TIMEOUT_MS`), streaming body included.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
/// Wait before the retry when the response has no usable `retry-after`.
pub const DEFAULT_RETRY_WAIT: Duration = Duration::from_secs(1);
/// Longest `retry-after` honoured; a longer one fails at once instead of waiting.
pub const MAX_RETRY_WAIT: Duration = Duration::from_secs(20);

/// The Claude API key every adapter reads per request (Phase 8: the user saves, changes or
/// removes it while the app runs, so it is shared, not copied). Never logged (no `Debug`).
#[derive(Clone, Default)]
pub struct SharedKey(Arc<RwLock<Option<String>>>);

impl SharedKey {
    /// A slot holding `key`, or empty for `None`.
    #[must_use]
    pub fn new(key: Option<String>) -> Self {
        Self(Arc::new(RwLock::new(key)))
    }

    /// Replaces the key (`None` removes it); every adapter sharing this slot sees it at once.
    pub fn replace(&self, key: Option<String>) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = key;
    }

    /// Whether a key is held.
    #[must_use]
    pub fn is_set(&self) -> bool {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
    }

    fn current(&self) -> Option<String> {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl From<&str> for SharedKey {
    fn from(key: &str) -> Self {
        Self::new(Some(key.to_owned()))
    }
}

/// Where and as whom requests are sent.
#[derive(Clone)]
pub struct ApiConfig {
    /// API origin, without a trailing slash (tests point it at a local server).
    pub base_url: String,
    /// The shared API key; a request made while it is empty carries no key (Anthropic answers 401).
    pub api_key: SharedKey,
}

impl ApiConfig {
    /// Production origin with the given shared key.
    #[must_use]
    pub fn new(api_key: SharedKey) -> Self {
        Self {
            base_url: API_BASE.to_owned(),
            api_key,
        }
    }

    /// A JSON `POST /v1/messages` with `body`.
    pub(crate) fn post_json(&self, client: &Client, body: &[u8]) -> RequestBuilder {
        self.request(client, reqwest::Method::POST, "/v1/messages")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_vec())
    }

    pub(crate) fn request(
        &self,
        client: &Client,
        method: reqwest::Method,
        path: &str,
    ) -> RequestBuilder {
        let builder = client.request(method, format!("{}{path}", self.base_url));
        match self.api_key.current() {
            Some(key) => with_key(builder, &key),
            None => builder.header("anthropic-version", ANTHROPIC_VERSION),
        }
    }
}

/// `builder` with the auth and version headers of `key`.
pub(crate) fn with_key(builder: RequestBuilder, key: &str) -> RequestBuilder {
    builder
        .header("x-api-key", key)
        .header("anthropic-version", ANTHROPIC_VERSION)
}

/// Builds the one client per app run: rustls, connect 10 s, whole request 120 s.
///
/// # Errors
/// `UPSTREAM_UNAVAILABLE` when the TLS backend cannot start.
pub fn build_client() -> Result<Client, AnthropicError> {
    Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| AnthropicError::from_reqwest(&e))
}

/// Whether a status gets the one retry.
#[must_use]
pub(crate) fn is_retryable(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

/// The wait before the retry: `retry-after` seconds when present and at most 20 s.
/// `None` means the server asked for longer than we wait.
pub(crate) fn retry_wait(response: &Response) -> Option<Duration> {
    let header = response.headers().get(reqwest::header::RETRY_AFTER);
    let Some(seconds) = header
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
    else {
        return Some(DEFAULT_RETRY_WAIT);
    };
    let wait = Duration::from_secs(seconds);
    (wait <= MAX_RETRY_WAIT).then_some(wait)
}

/// Sends a request built by `make`, retrying once on 408/429/5xx before any body is read.
/// Returns the successful response, or a typed error carrying the status and `error.type`.
pub(crate) async fn send(
    make: impl Fn() -> RequestBuilder,
    cancel: Option<&CancelToken>,
) -> Result<Response, AnthropicError> {
    let first = cancellable(make().send(), cancel)
        .await?
        .map_err(|e| AnthropicError::from_reqwest(&e))?;
    if !is_retryable(first.status()) {
        return check(first).await;
    }
    let Some(wait) = retry_wait(&first) else {
        return check(first).await;
    };
    drop(first);
    cancellable(tokio::time::sleep(wait), cancel).await?;
    let second = cancellable(make().send(), cancel)
        .await?
        .map_err(|e| AnthropicError::from_reqwest(&e))?;
    check(second).await
}

/// Runs `fut`, or fails with `CANCELLED` as soon as the token fires.
pub(crate) async fn cancellable<T>(
    fut: impl Future<Output = T>,
    cancel: Option<&CancelToken>,
) -> Result<T, AnthropicError> {
    let Some(token) = cancel else {
        return Ok(fut.await);
    };
    tokio::select! {
        biased;
        () = token.cancelled() => Err(AnthropicError::new(ErrorCode::Cancelled)),
        out = fut => Ok(out),
    }
}

async fn check(response: Response) -> Result<Response, AnthropicError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let code = if status == StatusCode::UNAUTHORIZED {
        ErrorCode::KeyRejected
    } else if is_retryable(status) {
        ErrorCode::UpstreamUnavailable
    } else {
        ErrorCode::UpstreamRejected
    };
    // The error body is read only for its `error.type`; its message is never logged.
    let body = response.bytes().await.unwrap_or_default();
    let error_type = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v["error"]["type"].as_str().map(str::to_owned));
    Err(AnthropicError {
        code,
        status: Some(status.as_u16()),
        error_type,
    })
}
