//! Driven adapters on the Claude Messages and Files APIs (docs/desktop/phases/phase-2-ask-flow.md
//! §2; decisions D-11 and D-12). Pure parts: [`request`] builds the exact bodies and [`sse`] parses
//! the stream. Async clients: [`AnthropicModel`], [`AnthropicFiles`], [`AnthropicFollowUps`] on one
//! reqwest client. [`bridge`] binds them to core's `AnswerModel`, `FollowUpSuggester` and
//! `RemoteFiles` ports; [`AnthropicKeyVerifier`] checks a pasted key (Phase 8).

pub mod bridge;
pub mod cancel;
pub mod error;
mod files;
pub mod followups;
mod http;
mod keys;
mod model;
pub mod request;
pub mod sse;
pub mod tools;

pub use bridge::{
    AnthropicAnswerModel, AnthropicFollowUpSuggester, AnthropicRemoteFiles, AnthropicRuntime,
};
pub use cancel::CancelToken;
pub use error::{AnthropicError, ErrorCode};
pub use files::AnthropicFiles;
pub use followups::AnthropicFollowUps;
pub use http::{ANTHROPIC_VERSION, API_BASE, ApiConfig, SharedKey, build_client};
pub use keys::AnthropicKeyVerifier;
pub use model::{AnthropicModel, StreamOutcome};
