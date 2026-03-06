use destack_lsp_server::{Cancellable, OngoingProgress, Unbounded, jsonrpc};
use destack_lsp_types as lsp;

use super::DestackLanguageServer;

/// Tracks work done progress for a request.
pub(super) struct WorkDoneProgressTracker {
    /// The work done progress token.
    token: Option<lsp::ProgressToken>,
    /// The active progress stream.
    progress: Option<OngoingProgress<Unbounded, Cancellable>>,
}

impl WorkDoneProgressTracker {
    /// Start a progress stream when a token is provided.
    pub(super) async fn start(
        server: &DestackLanguageServer,
        token: Option<lsp::ProgressToken>,
        title: &str,
        message: &str,
    ) -> Self {
        let mut progress = None;
        if let Some(token) = token.as_ref() {
            let _ = server
                .client
                .work_done_progress_create(lsp::WorkDoneProgressCreateParams {
                    token: token.clone(),
                })
                .await;
            progress = Some(
                server
                    .client
                    .progress(token.clone(), title.to_string())
                    .with_cancel_button()
                    .with_message(message)
                    .begin()
                    .await,
            );
        }

        Self { token, progress }
    }

    /// Report progress when available.
    pub(super) async fn report(&self, message: impl Into<String>) {
        if let Some(progress) = &self.progress {
            progress.report_with_message(message, None).await;
        }
    }

    /// Report progress at a chunk boundary and check for cancellation.
    pub(super) async fn report_chunk(
        &mut self,
        server: &DestackLanguageServer,
        processed: usize,
        chunk_size: usize,
        message: impl FnOnce(usize) -> String,
        cancel_message: &str,
    ) -> jsonrpc::Result<()> {
        if processed.is_multiple_of(chunk_size) {
            self.report(message(processed)).await;
        }

        // keep cancellation responsive for fast conversion loops on the async executor
        tokio::task::yield_now().await;
        self.check_cancelled(server, cancel_message).await
    }

    /// Check whether progress has been cancelled.
    pub(super) async fn check_cancelled(
        &mut self,
        server: &DestackLanguageServer,
        cancel_message: &str,
    ) -> jsonrpc::Result<()> {
        let Some(token) = self.token.as_ref() else {
            return Ok(());
        };

        if !server.is_progress_cancelled(token) {
            return Ok(());
        }

        if let Some(progress) = self.progress.take() {
            progress.finish_with_message(cancel_message).await;
        }
        server.clear_progress_cancel(token);
        Err(jsonrpc::Error::request_cancelled())
    }

    /// Finish the progress stream and clear cancellation state.
    pub(super) async fn finish(&mut self, server: &DestackLanguageServer, message: &str) {
        if let Some(progress) = self.progress.take() {
            progress.finish_with_message(message).await;
        }
        if let Some(token) = self.token.as_ref() {
            server.clear_progress_cancel(token);
        }
    }

    /// Finish a progress stream or return cancellation when the token was cancelled after the last chunk.
    pub(super) async fn finish_or_cancelled(
        &mut self,
        server: &DestackLanguageServer,
        finish_message: &str,
        cancel_message: &str,
    ) -> jsonrpc::Result<()> {
        self.check_cancelled(server, cancel_message).await?;
        self.finish(server, finish_message).await;
        Ok(())
    }
}
