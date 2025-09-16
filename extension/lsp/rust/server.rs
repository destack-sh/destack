//! LSP server implementation.

use std::collections::HashMap;

use dyst_language_source::Uri;

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
        // semantic tokens
        let semantic_tokens_legend = lsp::SemanticTokensLegend {
            token_types: SEMANTIC_TOKEN_TYPES.to_vec(),
            token_modifiers: vec![],
        };

        // file watching patterns for Dyst files
        let file_operations = lsp::FileOperationRegistrationOptions {
            filters: vec![lsp::FileOperationFilter {
                scheme: Some("file".to_string()),
                pattern: lsp::FileOperationPattern {
                    glob: "**/*.ds".to_string(),
                    matches: Some(lsp::FileOperationPatternKind::File),
                    options: None,
                },
            }],
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
            workspace: Some(lsp::WorkspaceServerCapabilities {
                workspace_folders: Some(lsp::WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(lsp::OneOf::Left(true)),
                }),
                file_operations: Some(lsp::WorkspaceFileOperationsServerCapabilities {
                    did_create: Some(file_operations.clone()),
                    will_create: None,
                    did_rename: Some(file_operations.clone()),
                    will_rename: None,
                    did_delete: Some(file_operations.clone()),
                    will_delete: None,
                }),
            }),
            ..Default::default()
        };

        self.client
            .log_message(MessageType::INFO, "destack: initialize");

        Ok(lsp::InitializeResult {
            capabilities,
            server_info: Some(lsp::ServerInfo {
                name: "destack".to_string(),
                version: None,
            }),
        })
    }

    /// Handle the initialized notification from the client.
    fn initialized(&self, params: lsp::InitializedParams) {
        // register for watching .ds files
        let watchers = vec![lsp::FileSystemWatcher {
            glob_pattern: lsp::GlobPattern::String("**/*.ds".to_string()),
            kind: Some(lsp::WatchKind::all()),
        }];
        let registration = lsp::Registration {
            id: "watch-destack-files".to_string(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options: Some(
                serde_json::to_value(lsp::DidChangeWatchedFilesRegistrationOptions { watchers })
                    .unwrap(),
            ),
        };

        // send registration request to client
        let _ = self.client.register_capability(lsp::RegistrationParams {
            registrations: vec![registration],
        });

        self.client
            .log_message(MessageType::INFO, "destack: initialized");
    }

    /// Gracefully shut down the server.
    fn shutdown(&self) -> JsonRpcResult<()> {
        self.client
            .log_message(MessageType::INFO, "destack: shutdown");
        Ok(())
    }

    /// Handle text document open notifications.
    fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.client
            .log_message(MessageType::INFO, format!("destack: did_open: {}", uri));
    }

    /// Handle text document change notifications.
    fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.client
            .log_message(MessageType::INFO, format!("destack: did_change: {}", uri));
    }

    /// Handle text document close notifications.
    fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.client
            .log_message(MessageType::INFO, format!("destack: did_close: {}", uri));
    }

    /// Handle watched file change notifications.
    fn did_change_watched_files(&self, params: lsp::DidChangeWatchedFilesParams) {
        for change in params.changes {
            let uri = change.uri.to_string();

            self.client.log_message(
                MessageType::INFO,
                format!("destack: file changed: {} (type: {:?})", uri, change.typ),
            );

            match change.typ {
                lsp::FileChangeType::CREATED => {
                    // file was created, we might want to add it to workspace
                    self.client
                        .log_message(MessageType::INFO, format!("destack: file created: {}", uri));
                }
                lsp::FileChangeType::CHANGED => {
                    // file was changed externally, reload if we have it open
                    let workspace_root = self.find_workspace_root(&uri);
                    if let Some(workspace) = self.workspaces_by_uri.get(&workspace_root) {
                        if workspace
                            .get_source(Uri::from_string(change.uri.to_string()))
                            .is_some()
                        {
                            // TODO @Incomplete: reload file content from disk
                            self.client.log_message(
                                MessageType::INFO,
                                format!("destack: external change to tracked file: {}", uri),
                            );
                        }
                    }
                }
                lsp::FileChangeType::DELETED => {
                    // file was deleted, remove from workspace
                    self.client
                        .log_message(MessageType::INFO, format!("destack: file deleted: {}", uri));
                }
                _ => {}
            }
        }
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

impl DestackLanguageServer {
    /// Find the workspace root for a given file URI.
    fn find_workspace_root(&self, uri: &str) -> String {
        // TODO @Incomplete: implement proper workspace root detection
        // for now, just use the directory containing the file
        if let Some(last_slash) = uri.rfind('/') {
            uri[..last_slash].to_string()
        } else {
            uri.to_string()
        }
    }
}
