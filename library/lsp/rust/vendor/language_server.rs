//! Define the synchronous `LanguageServer` trait.

use crate::vendor::{jsonrpc, lsp_types};

/// Implement a synchronous Language Server.
///
/// Methods mirror the LSP request/notification surface.
pub trait LanguageServer: Send + Sync + 'static {
    fn initialize(
        &self,
        _params: lsp_types::InitializeParams,
    ) -> jsonrpc::Result<lsp_types::InitializeResult> {
        Ok(lsp_types::InitializeResult {
            capabilities: lsp_types::ServerCapabilities::default(),
            server_info: None,
        })
    }

    fn initialized(&self, _params: lsp_types::InitializedParams) {}

    fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    fn did_open(&self, _params: lsp_types::DidOpenTextDocumentParams) {}

    fn did_change(&self, _params: lsp_types::DidChangeTextDocumentParams) {}

    fn semantic_tokens_full(
        &self,
        _params: lsp_types::SemanticTokensParams,
    ) -> jsonrpc::Result<Option<lsp_types::SemanticTokensResult>> {
        Ok(None)
    }

    fn semantic_tokens_range(
        &self,
        _params: lsp_types::SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<lsp_types::SemanticTokensRangeResult>> {
        Ok(None)
    }

    fn hover(&self, _params: lsp_types::HoverParams) -> jsonrpc::Result<Option<lsp_types::Hover>> {
        Ok(None)
    }
}
