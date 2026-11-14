//! The core port adapters: bind the async Anthropic clients to core's synchronous ports. Every
//! call runs on one dedicated tokio runtime owned by [`AnthropicRuntime`], so callers must be on a
//! plain or `spawn_blocking` thread (the use cases run there), never inside an async task.

use crate::cancel::CancelToken;
use crate::error::AnthropicError;
use crate::files::AnthropicFiles;
use crate::followups::AnthropicFollowUps;
use crate::model::AnthropicModel;
use crate::request::messages_body;
use crate::sse::StreamEvent;
use crate::tools::tool_call;
use kristoff_contracts::model_config::answer_system_prompt;
use kristoff_core::domain::model_request::{AnswerEnd, FollowUps, ModelEvent, ModelRequest};
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::ports::{AnswerModel, CancelSignal, FollowUpSuggester, Logger, RemoteFiles};
use std::sync::Arc;
use std::time::Duration;

/// How often a running answer checks the core cancel signal.
const CANCEL_POLL: Duration = Duration::from_millis(50);

/// The runtime every Anthropic call runs on (2 worker threads).
pub struct AnthropicRuntime {
    rt: Option<tokio::runtime::Runtime>,
}

impl AnthropicRuntime {
    /// Starts the runtime.
    ///
    /// # Errors
    /// `INTERNAL` when the threads cannot start.
    pub fn new() -> AppResult<Self> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("kristoff-anthropic")
            .enable_all()
            .build()
            .map_err(|e| AppError::new(ErrorCode::Internal, format!("runtime: {e}")))?;
        Ok(Self { rt: Some(rt) })
    }

    pub(crate) fn block_on<F: Future>(&self, future: F) -> F::Output {
        match &self.rt {
            Some(rt) => rt.block_on(future),
            None => unreachable_runtime(),
        }
    }
}

impl Drop for AnthropicRuntime {
    fn drop(&mut self) {
        // `shutdown_background` never blocks or panics, even when dropped from an async context.
        if let Some(rt) = self.rt.take() {
            rt.shutdown_background();
        }
    }
}

/// The runtime is only taken in `drop`, after which no method can run.
fn unreachable_runtime() -> ! {
    std::process::abort()
}

/// The core error of an adapter failure: a rejected key is `KEY_INVALID` (the user must fix it in
/// Settings); everything else is `UPSTREAM_UNAVAILABLE`.
pub(crate) fn to_app(e: &AnthropicError) -> AppError {
    if e.code == crate::error::ErrorCode::KeyRejected {
        return AppError::new(
            ErrorCode::KeyInvalid,
            "Anthropic rejected the Claude API key",
        );
    }
    AppError::new(
        ErrorCode::UpstreamUnavailable,
        format!("the Claude API call failed: {e}"),
    )
}

/// `AnswerModel` on the streaming Messages API. The system prompt is `kristoff-contracts`'
/// `answer_system_prompt` (with the statute addition when the tools are offered).
pub struct AnthropicAnswerModel {
    rt: Arc<AnthropicRuntime>,
    model: AnthropicModel,
}

impl AnthropicAnswerModel {
    /// Binds `model` to the runtime.
    #[must_use]
    pub fn new(rt: Arc<AnthropicRuntime>, model: AnthropicModel) -> Self {
        Self { rt, model }
    }
}

/// Completes once `cancel` fires (polled), after cancelling `token`.
async fn watch_cancel(cancel: &dyn CancelSignal, token: &CancelToken) {
    while !cancel.is_cancelled() {
        tokio::time::sleep(CANCEL_POLL).await;
    }
    token.cancel();
    std::future::pending::<()>().await;
}

impl AnswerModel for AnthropicAnswerModel {
    fn stream(
        &self,
        request: &ModelRequest,
        cancel: &dyn CancelSignal,
        on_event: &mut dyn FnMut(ModelEvent),
    ) -> AppResult<AnswerEnd> {
        if cancel.is_cancelled() {
            return Err(AppError::new(
                ErrorCode::UpstreamUnavailable,
                "the answer was cancelled",
            ));
        }
        let body = messages_body(
            request,
            &answer_system_prompt(request.statutes_on, request.cases_on),
        );
        let token = CancelToken::new();
        let mut tool_calls = Vec::new();
        let mut forward = |event: &StreamEvent| match event {
            StreamEvent::TextDelta(text) => on_event(ModelEvent::Text(text.clone())),
            StreamEvent::CitationDelta(source) => on_event(ModelEvent::Citation(source.clone())),
            StreamEvent::ToolUse { id, name, input } => tool_calls.push(tool_call(id, name, input)),
            _ => {}
        };
        let outcome = self.rt.block_on(async {
            tokio::select! {
                out = self.model.stream(&body, &token, &mut forward) => out,
                () = watch_cancel(cancel, &token) => Err(AnthropicError::cancelled()),
            }
        });
        let outcome = outcome.map_err(|e| to_app(&e))?;
        Ok(AnswerEnd {
            usage: outcome.usage,
            stop_reason: outcome.stop_reason,
            tool_calls,
            assistant_content_json: serde_json::Value::Array(outcome.content).to_string(),
        })
    }
}

/// `FollowUpSuggester` on one Haiku call; failures are logged and give no questions.
pub struct AnthropicFollowUpSuggester {
    rt: Arc<AnthropicRuntime>,
    inner: AnthropicFollowUps,
    log: Arc<dyn Logger>,
}

impl AnthropicFollowUpSuggester {
    /// Binds `inner` to the runtime; failures go to `log`.
    #[must_use]
    pub fn new(rt: Arc<AnthropicRuntime>, inner: AnthropicFollowUps, log: Arc<dyn Logger>) -> Self {
        Self { rt, inner, log }
    }
}

impl FollowUpSuggester for AnthropicFollowUpSuggester {
    fn suggest(&self, question: &str, answer_excerpt: &str) -> FollowUps {
        let suggestion = self
            .rt
            .block_on(self.inner.suggest(question, answer_excerpt));
        if let Some(reason) = suggestion.failure {
            self.log.warn("followups.failed", reason);
        }
        suggestion.follow_ups
    }
}

/// `RemoteFiles` on the Files API; delete failures are logged.
pub struct AnthropicRemoteFiles {
    rt: Arc<AnthropicRuntime>,
    files: AnthropicFiles,
    log: Arc<dyn Logger>,
}

impl AnthropicRemoteFiles {
    /// Binds `files` to the runtime; delete failures go to `log`.
    #[must_use]
    pub fn new(rt: Arc<AnthropicRuntime>, files: AnthropicFiles, log: Arc<dyn Logger>) -> Self {
        Self { rt, files, log }
    }
}

impl RemoteFiles for AnthropicRemoteFiles {
    fn upload(&self, file_name: &str, pdf: &[u8]) -> AppResult<String> {
        self.rt
            .block_on(self.files.upload(file_name, pdf))
            .map_err(|e| to_app(&e))
    }

    fn delete(&self, file_ids: &[String]) {
        for id in file_ids {
            if let Err(e) = self.rt.block_on(self.files.delete(id)) {
                self.log.warn("files.delete_failed", e.code.as_str());
            }
        }
    }
}
