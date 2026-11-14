//! The Claude-facing ports the composition binds: the answer model, the follow-up suggester, the
//! Files API, the PDF splitter and the key check. Production builds them on the Anthropic adapters
//! around one shared key slot that the user fills in Settings (Phase 8: nothing is baked in);
//! tests inject fakes.

use kristoff_adapters::log::StderrLogger;
use kristoff_anthropic::{
    API_BASE, AnthropicAnswerModel, AnthropicFiles, AnthropicFollowUpSuggester, AnthropicFollowUps,
    AnthropicKeyVerifier, AnthropicModel, AnthropicRemoteFiles, AnthropicRuntime, ApiConfig,
    SharedKey, build_client,
};
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::ports::{
    AnswerModel, FollowUpSuggester, KeyVerifier, Logger, PdfSplitter, RemoteFiles,
};
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
    /// Checks a pasted key before it is saved.
    pub verifier: Box<dyn KeyVerifier>,
    /// The key every Claude call reads; the composition fills, replaces and clears it.
    pub key: SharedKey,
}

impl AiPorts {
    /// The production ports: one runtime and one HTTP client for every Claude call, all reading
    /// the key from `key` (empty until the user saves one).
    ///
    /// # Errors
    /// `INTERNAL` when the runtime or the TLS client cannot start.
    pub fn anthropic(key: SharedKey) -> AppResult<Self> {
        let rt = Arc::new(AnthropicRuntime::new()?);
        let client =
            build_client().map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?;
        let verifier = AnthropicKeyVerifier::new(Arc::clone(&rt), client.clone(), API_BASE);
        let config = ApiConfig::new(key.clone());
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
            verifier: Box::new(verifier),
            key,
        })
    }
}
