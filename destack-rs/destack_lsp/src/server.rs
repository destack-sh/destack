//! LSP server implementation.

use crate::doc_store::DocumentStore;
use crate::semantic::compute_semantic_tokens;
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::{Client, LanguageServer, lsp_types as lsp};

#[derive(Debug, Clone)]
pub struct Backend {
    pub client: Client,
    pub docs: DocumentStore,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: lsp::InitializeParams) -> JsonRpcResult<lsp::InitializeResult> {
        let semantic_tokens_legend = lsp::SemanticTokensLegend {
            token_types: vec![
                // map categories to VS Code semantic token types
                lsp::SemanticTokenType::COMMENT,
                lsp::SemanticTokenType::KEYWORD,
                lsp::SemanticTokenType::STRING,
                lsp::SemanticTokenType::NUMBER,
                lsp::SemanticTokenType::OPERATOR,
                lsp::SemanticTokenType::FUNCTION, // use function color for punctuation to differentiate
                lsp::SemanticTokenType::TYPE,
                lsp::SemanticTokenType::VARIABLE,
            ],
            token_modifiers: vec![],
        };

        let capabilities = lsp::ServerCapabilities {
            text_document_sync: Some(lsp::TextDocumentSyncCapability::Kind(
                lsp::TextDocumentSyncKind::FULL,
            )),
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
                name: "destack_lsp".to_string(),
                version: None,
            }),
        })
    }

    async fn initialized(&self, _: lsp::InitializedParams) {}

    async fn shutdown(&self) -> JsonRpcResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.docs.set(&uri, text);
    }

    async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        // SyncKind::FULL, take the full content from the single change
        if let Some(change) = params.content_changes.into_iter().last() {
            self.docs.set(&uri, change.text);
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: lsp::SemanticTokensParams,
    ) -> JsonRpcResult<Option<lsp::SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let Some(text) = self.docs.get(&uri) else {
            return Ok(None);
        };
        let tokens = compute_semantic_tokens(&text);
        Ok(Some(lsp::SemanticTokensResult::Tokens(
            lsp::SemanticTokens {
                result_id: None,
                data: tokens,
            },
        )))
    }

    async fn semantic_tokens_range(
        &self,
        params: lsp::SemanticTokensRangeParams,
    ) -> JsonRpcResult<Option<lsp::SemanticTokensRangeResult>> {
        let uri = params.text_document.uri;
        let Some(text) = self.docs.get(&uri) else {
            return Ok(None);
        };
        let tokens = compute_semantic_tokens(&text);
        Ok(Some(lsp::SemanticTokensRangeResult::Tokens(
            lsp::SemanticTokens {
                result_id: None,
                data: tokens,
            },
        )))
    }
}

// re-export doc_store in lib.rs instead
