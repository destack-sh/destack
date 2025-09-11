//! LSP server implementation.

use dyst_language_ast::{BlockFormat, Parser};
use dyst_language_source::{Source, SourceId};
use dyst_language_token::TokenType;

use crate::doc::DocumentStore;
use crate::protocol::jsonrpc::Result as JsonRpcResult;
use crate::protocol::types::MessageType;
use crate::protocol::{Client, LanguageServer, lsp};
use crate::semantic::{SEMANTIC_TOKEN_TYPES, get_semantic_tokens};

#[derive(Debug, Clone)]
pub struct DestackLanguageServer {
    pub client: Client,
    pub docs: DocumentStore,
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

    fn hover(&self, _params: lsp::HoverParams) -> JsonRpcResult<Option<lsp::Hover>> {
        // get parsed document
        // nocheckin todo!: parse all docs in workspace
        let text = self
            .docs
            .get(&_params.text_document_position_params.text_document.uri)
            .unwrap_or_default();
        let source = Source::from_string(SourceId::new(0), "<semantic>".to_string(), text.clone());
        let mut parser = Parser::from_source(&source);
        let _ = parser.with_recovery(
            parser.mark(),
            |parser| parser.eat_block_body(BlockFormat::Implicit),
            Vec::new(),
            TokenType::End,
        );
        parser.process_annotations();

        // nocheckin todo!: generalize NodeMap / mapping stuff (for LSP or maybe overall)
        // compute line starts for mapping
        let mut line_starts: Vec<usize> = vec![0];
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                line_starts.push(i + 1);
            }
        }

        // map LSP position (UTF-16 column) to byte offset in text
        let position = _params.text_document_position_params.position;
        let line_index = position.line as usize;
        if line_index >= line_starts.len() {
            return Ok(None);
        }
        let line_start = line_starts[line_index];
        let line_end = if line_index + 1 < line_starts.len() {
            line_starts[line_index + 1] - 1 // exclude the newline
        } else {
            text.len()
        };
        let target_utf16_col = position.character as usize;
        let mut byte_cursor = line_start;
        let mut utf16_col_so_far: usize = 0;
        for (idx, ch) in text[line_start..line_end].char_indices() {
            if utf16_col_so_far >= target_utf16_col {
                break;
            }
            utf16_col_so_far += ch.encode_utf16(&mut [0u16; 2]).len();
            byte_cursor = line_start + idx + ch.len_utf8();
        }
        // if target col points exactly at start of line, keep line_start
        let byte_offset = if target_utf16_col == 0 {
            line_start
        } else {
            byte_cursor
        };

        // get hover
        if let Some((_, _)) = parser.tree.map.get_enclosing_span(byte_offset as u32) {
            // ... todo!
        }

        Ok(None)
    }
}
