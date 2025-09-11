//! LSP server implementation.

use crate::doc::DocumentStore;
use crate::protocol::jsonrpc::Result as JsonRpcResult;
use crate::protocol::types::MessageType;
use crate::protocol::{Client, LanguageServer, lsp};
use crate::semantic::{get_semantic_tokens, get_token_type_at_position};

#[derive(Debug, Clone)]
pub struct Backend {
    pub client: Client,
    pub docs: DocumentStore,
}

impl Backend {}

impl LanguageServer for Backend {
    /// Initialize the language server with client capabilities and return server capabilities.
    fn initialize(&self, _: lsp::InitializeParams) -> JsonRpcResult<lsp::InitializeResult> {
        self.client
            .log_message(MessageType::INFO, "destack: initialize");

        // semantic tokens
        let semantic_tokens_legend = lsp::SemanticTokensLegend {
            token_types: vec![
                lsp::SemanticTokenType::COMMENT,
                lsp::SemanticTokenType::DECORATOR,
                lsp::SemanticTokenType::KEYWORD,
                lsp::SemanticTokenType::STRING,
                lsp::SemanticTokenType::NUMBER,
                lsp::SemanticTokenType::OPERATOR,
                lsp::SemanticTokenType::FUNCTION,
                lsp::SemanticTokenType::TYPE,
                lsp::SemanticTokenType::VARIABLE,
            ],
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

    /// Handle document open notification and store the document content.
    fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.docs.set(&uri, text);
        self.client
            .log_message(MessageType::INFO, format!("destack: did_open: {:?}", uri));
    }

    /// Handle document change notification and update the stored document content.
    fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        // SyncKind::FULL, take the full content from the single change
        if let Some(change) = params.content_changes.into_iter().last() {
            self.docs.set(&uri, change.text);
        }
        self.client
            .log_message(MessageType::INFO, format!("destack: did_change: {:?}", uri));
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
        let Some(text) = self.docs.get(&uri) else {
            return Ok(None);
        };
        let tokens = get_semantic_tokens(&text);
        self.client.log_message(
            MessageType::INFO,
            format!(
                "destack: semantic_tokens_full: {:?} -> {:?}",
                uri,
                tokens.len()
            ),
        );
        Ok(Some(lsp::SemanticTokensResult::Tokens(
            lsp::SemanticTokens {
                result_id: None,
                data: tokens,
            },
        )))
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
        let Some(text) = self.docs.get(&uri) else {
            return Ok(None);
        };
        let tokens = get_semantic_tokens(&text);
        self.client.log_message(
            MessageType::INFO,
            format!(
                "destack: semantic_tokens_range: {:?} -> {:?}",
                uri,
                tokens.len()
            ),
        );
        Ok(Some(lsp::SemanticTokensRangeResult::Tokens(
            lsp::SemanticTokens {
                result_id: None,
                data: tokens,
            },
        )))
    }

    /// Provide hover information for a symbol at a given position.
    fn hover(&self, params: lsp::HoverParams) -> JsonRpcResult<Option<lsp::Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        self.client.log_message(
            MessageType::INFO,
            format!("destack: hover: {:?} @ {:?}", uri, position),
        );
        let Some(text) = self.docs.get(&uri) else {
            return Ok(None);
        };

        if let Some(kind) = get_token_type_at_position(&text, &position) {
            let markdown = lsp::MarkedString::String(format!("token: {:?}", kind));
            let hover = lsp::Hover {
                contents: lsp::HoverContents::Scalar(markdown),
                range: None,
            };
            return Ok(Some(hover));
        }

        Ok(None)
    }
}
