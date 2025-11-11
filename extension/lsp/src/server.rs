use tower_lsp_server::{Client, LanguageServer, jsonrpc, lsp_types as lsp};

use crate::semantic;
use crate::workspace::TRACKED_FILE_TYPES;

#[derive(Debug)]
pub struct DestackLanguageServer {
    /// The client that the language server is connected to.
    pub(super) client: Client,
}

impl DestackLanguageServer {
    /// Create a new language server instance with the given client.
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

impl LanguageServer for DestackLanguageServer {
    // ------------------------------------------------------------
    // Lifecycle
    // ------------------------------------------------------------

    /// The [`initialize`] request is the first request sent from the client to the server.
    async fn initialize(
        &self,
        _params: lsp::InitializeParams,
    ) -> jsonrpc::Result<lsp::InitializeResult> {
        self.client
            .log_message(lsp::MessageType::INFO, "destack.initialize.start")
            .await;

        let file_operation_filters: Vec<lsp::FileOperationFilter> = TRACKED_FILE_TYPES
            .iter()
            .map(|file_type| lsp::FileOperationFilter {
                scheme: None,
                pattern: lsp::FileOperationPattern {
                    glob: file_type.glob().expect("unknown file type").to_string(),
                    matches: Some(lsp::FileOperationPatternKind::File),
                    options: Some(lsp::FileOperationPatternOptions {
                        ignore_case: Some(true),
                    }),
                },
            })
            .collect();

        let capabilities = lsp::ServerCapabilities {
            text_document_sync: Some(lsp::TextDocumentSyncCapability::Kind(
                lsp::TextDocumentSyncKind::FULL,
            )),
            semantic_tokens_provider: Some(
                lsp::SemanticTokensServerCapabilities::SemanticTokensOptions(
                    lsp::SemanticTokensOptions {
                        legend: semantic::legend(),
                        range: Some(true),
                        full: Some(lsp::SemanticTokensFullOptions::Bool(true)),
                        work_done_progress_options: lsp::WorkDoneProgressOptions::default(),
                    },
                ),
            ),
            document_formatting_provider: Some(lsp::OneOf::Left(true)),
            workspace: Some(lsp::WorkspaceServerCapabilities {
                workspace_folders: Some(lsp::WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(lsp::OneOf::Left(true)),
                }),
                file_operations: Some(lsp::WorkspaceFileOperationsServerCapabilities {
                    did_create: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    did_rename: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    did_delete: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    will_create: None,
                    will_rename: None,
                    will_delete: None,
                }),
            }),
            ..Default::default()
        };

        self.client
            .log_message(lsp::MessageType::INFO, "destack.initialize")
            .await;

        Ok(lsp::InitializeResult {
            capabilities,
            server_info: Some(lsp::ServerInfo {
                name: "destack".to_string(),
                version: None,
            }),
        })
    }

    /// The [`initialized`] notification is sent from the client to the server after the client received the result of the initialize request but before the client sends anything else.
    async fn initialized(&self, _: lsp::InitializedParams) {
        todo!("initialized");
    }

    /// The [`shutdown`] request asks the server to gracefully shut down, but to not exit.
    async fn shutdown(&self) -> jsonrpc::Result<()> {
        self.client
            .log_message(lsp::MessageType::INFO, "destack.shutdown")
            .await;
        Ok(())
    }

    // ------------------------------------------------------------
    // Synchronization
    // ------------------------------------------------------------

    /// The [`textDocument/didOpen`] notification is sent from the client to the server to signal that a new text document has been opened by the client.
    async fn did_open(&self, _params: lsp::DidOpenTextDocumentParams) {
        todo!("did_open");
    }

    /// The [`textDocument/didChange`] notification is sent from the client to the server to signal changes to a text document.
    async fn did_change(&self, _params: lsp::DidChangeTextDocumentParams) {
        todo!("did_change");
    }

    /// The [`textDocument/didClose`] notification is sent from the client to the server when the document got closed in the client.
    async fn did_close(&self, _params: lsp::DidCloseTextDocumentParams) {
        todo!("did_close");
    }

    /// The [`workspace/didChangeWorkspaceFolders`] notification is sent from the client to the server to inform about workspace folder configuration changes.
    async fn did_change_workspace_folders(&self, _params: lsp::DidChangeWorkspaceFoldersParams) {
        todo!("did_change_workspace_folders");
    }

    /// The [`workspace/didChangeWatchedFiles`] notification is sent from the client to the server when the client detects changes to files watched by the language client.
    async fn did_change_watched_files(&self, _params: lsp::DidChangeWatchedFilesParams) {
        todo!("did_change_watched_files");
    }

    /// The [`workspace/didCreateFiles`] notification is sent from the client to the server after files are created.
    async fn did_create_files(&self, _params: lsp::CreateFilesParams) {
        todo!("did_create_files");
    }

    /// The [`workspace/didRenameFiles`] notification is sent from the client to the server after files are renamed.
    async fn did_rename_files(&self, _params: lsp::RenameFilesParams) {
        todo!("did_rename_files");
    }

    /// The [`workspace/didDeleteFiles`] notification is sent from the client to the server after files are deleted.
    async fn did_delete_files(&self, _params: lsp::DeleteFilesParams) {
        todo!("did_delete_files");
    }

    // ------------------------------------------------------------
    // Semantic Tokens
    // ------------------------------------------------------------

    /// The [`textDocument/semanticTokens/full`] request is sent from the client to the server to
    /// resolve the semantic tokens of a given file.
    async fn semantic_tokens_full(
        &self,
        _params: lsp::SemanticTokensParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensResult>> {
        todo!("semantic_tokens_full");
    }

    /// The [`textDocument/semanticTokens/range`] request is sent from the client to the server to
    /// resolve the semantic tokens **for the visible range** of a given file.
    async fn semantic_tokens_range(
        &self,
        _params: lsp::SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensRangeResult>> {
        todo!("semantic_tokens_range");
    }

    // ------------------------------------------------------------
    // Formatting
    // ------------------------------------------------------------

    /// The [`textDocument/formatting`] request is sent from the client to the server to
    /// format a given text document.
    async fn formatting(
        &self,
        _params: lsp::DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        todo!("formatting");
    }
}
