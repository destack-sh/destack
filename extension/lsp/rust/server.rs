//! LSP server implementation.

use std::collections::HashMap;

use crate::protocol::jsonrpc::Result as JsonRpcResult;
use crate::protocol::types::MessageType;
use crate::protocol::{Client, LanguageServer, lsp};
use crate::semantic::SEMANTIC_TOKEN_TYPES;
use crate::workspace::Workspace;

#[derive(Debug)]
pub struct DestackLanguageServer {
    pub client: Client,
    pub workspaces_by_uri: HashMap<String, Workspace>,
}

impl DestackLanguageServer {}

impl LanguageServer for DestackLanguageServer {
    /// Initialize the language server with client capabilities and return server capabilities.
    fn initialize(&self, _: lsp::InitializeParams) -> JsonRpcResult<lsp::InitializeResult> {
        self.client
            .log_message(MessageType::INFO, "destack: initialize");

        // semantic tokens
        let semantic_tokens_legend = lsp::SemanticTokensLegend {
            token_types: SEMANTIC_TOKEN_TYPES.to_vec(),
            token_modifiers: vec![],
        };

        // capabilities
        let capabilities = lsp::ServerCapabilities {
            text_document_sync: Some(lsp::TextDocumentSyncCapability::Kind(
                lsp::TextDocumentSyncKind::FULL,
            )),
            hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
            semantic_tokens_provider: Some(
                lsp::SemanticTokensServerCapabilities::SemanticTokensOptions(
                    lsp::SemanticTokensOptions {
                        legend: semantic_tokens_legend,
                        range: Some(true),
                        full: Some(lsp::SemanticTokensFullOptions::Bool(true)),
                        work_done_progress_options: lsp::WorkDoneProgressOptions::default(),
                    },
                ),
            ),
            ..Default::default()
        };

        Ok(lsp::InitializeResult {
            capabilities,
            server_info: Some(lsp::ServerInfo {
                name: "destack".to_string(),
                version: None,
            }),
        })
    }

    /// Handle the initialized notification from the client.
    fn initialized(&self, _: lsp::InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "destack: initialized");
    }

    /// Gracefully shut down the server.
    fn shutdown(&self) -> JsonRpcResult<()> {
        self.client
            .log_message(MessageType::INFO, "destack: shutdown");
        Ok(())
    }

    /// Compute semantic tokens for the entire document.
    fn semantic_tokens_full(
        &self,
        params: lsp::SemanticTokensParams,
    ) -> JsonRpcResult<Option<lsp::SemanticTokensResult>> {
        let uri = params.text_document.uri;
        self.client.log_message(
            MessageType::INFO,
            format!("destack: semantic_tokens_full: {:?}", uri),
        );
        Ok(None)
    }

    /// Compute semantic tokens for a specific range in the document.
    fn semantic_tokens_range(
        &self,
        params: lsp::SemanticTokensRangeParams,
    ) -> JsonRpcResult<Option<lsp::SemanticTokensRangeResult>> {
        let uri = params.text_document.uri;
        self.client.log_message(
            MessageType::INFO,
            format!("destack: semantic_tokens_range: {:?}", uri),
        );
        Ok(None)
    }
}
