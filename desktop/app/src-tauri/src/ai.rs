//! The Claude-facing ports the composition binds: the answer model, the follow-up suggester, the
//! Files API and the PDF splitter. Production builds them on the Anthropic adapters with the baked
//! key; tests inject fakes.

use kristoff_adapters::log::StderrLogger;
use kristoff_anthropic::{
    AnthropicAnswerModel, AnthropicFiles, AnthropicFollowUpSuggester, AnthropicFollowUps,
    AnthropicModel, AnthropicRemoteFiles, AnthropicRuntime, ApiConfig, build_client,
};
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::ports::{AnswerModel, FollowUpSuggester, Logger, PdfSplitter, RemoteFiles};
use kristoff_pdf::LopdfSplitter;
use std::sync::Arc;

/// The bound AI and processing ports.
pub struct AiPorts {
    /// Streams answers.
    pub model: Box<dyn AnswerModel>,
    /// Suggests follow-ups.
    pub follow_ups: Box<dyn FollowUpSuggester>,
    /// The Files API.
    pub remote: Box<dyn RemoteFiles>,
    /// Counts and cuts PDFs.
    pub splitter: Box<dyn PdfSplitter>,
    /// Warnings about best-effort steps and background failures.
    pub log: Arc<dyn Logger>,
}

impl AiPorts {
    /// The production ports: one runtime and one HTTP client for every Claude call.
    ///
    /// # Errors
    /// `INTERNAL` for an empty key or when the runtime or the TLS client cannot start.
    pub fn anthropic(api_key: &str) -> AppResult<Self> {
        if api_key.trim().is_empty() {
            return Err(AppError::new(
                ErrorCode::Internal,
                "the Claude API key is missing",
            ));
        }
        let rt = Arc::new(AnthropicRuntime::new()?);
        let client =
            build_client().map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?;
        let config = ApiConfig::new(api_key.to_owned());
        let log: Arc<dyn Logger> = Arc::new(StderrLogger);
        let model = AnthropicModel::new(client.clone(), config.clone());
        let follow_ups = AnthropicFollowUps::new(client.clone(), config.clone());
        let files = AnthropicFiles::new(client, config);
        Ok(Self {
            model: Box::new(AnthropicAnswerModel::new(Arc::clone(&rt), model)),
            follow_ups: Box::new(AnthropicFollowUpSuggester::new(
                Arc::clone(&rt),
                follow_ups,
                Arc::clone(&log),
            )),
            remote: Box::new(AnthropicRemoteFiles::new(rt, files, Arc::clone(&log))),
            splitter: Box::new(LopdfSplitter),
            log,
        })
    }
}
