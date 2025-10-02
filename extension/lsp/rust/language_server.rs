//! Async language server implementation built on tower-lsp-server.

use std::str::FromStr;

use tower_lsp_server::{LanguageServer, jsonrpc};

use dyst_source::SourceFormat;

use crate::document::DocumentContent;
use crate::workspace::{TRACKED_FORMATS, infer_source_format_from_lsp_uri, lsp_uri_to_uri};
use crate::{DestackLanguageServer, semantic};
use tower_lsp_server::lsp_types as lsp;

impl LanguageServer for DestackLanguageServer {
    // ------------------------------------------------------------
    // Lifecycle
    // ------------------------------------------------------------

    /// The [`initialize`] request is the first request sent from the client to the server.
    async fn initialize(
        &self,
        params: lsp::InitializeParams,
    ) -> jsonrpc::Result<lsp::InitializeResult> {
        self.client
            .log_message(lsp::MessageType::INFO, "destack.initialize.start")
            .await;

        self.add_initial_workspaces(&params).await;

        let file_operation_filters: Vec<lsp::FileOperationFilter> = TRACKED_FORMATS
            .iter()
            .map(|format| lsp::FileOperationFilter {
                scheme: None,
                pattern: lsp::FileOperationPattern {
                    glob: format.glob().to_string(),
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
        self.client
            .log_message(lsp::MessageType::INFO, "destack.initialized.start")
            .await;
        self.register_all_file_watches().await;
        self.reindex_all_workspaces().await;
        self.client
            .log_message(lsp::MessageType::INFO, "destack.initialized")
            .await;
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
    async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let format = infer_source_format_from_lsp_uri(&uri).unwrap_or(SourceFormat::Dyst);
        self.upsert_open_text_document(&uri, format, params.text_document.text)
            .await;
        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack.did_open uri={}", uri.as_str()),
            )
            .await;
    }

    /// The [`textDocument/didChange`] notification is sent from the client to the server to signal changes to a text document.
    async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            let format = infer_source_format_from_lsp_uri(&uri).unwrap_or(SourceFormat::Dyst);
            self.upsert_open_text_document(&uri, format, change.text)
                .await;
        }
        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack.did_change uri={}", uri.as_str()),
            )
            .await;
    }

    /// The [`textDocument/didClose`] notification is sent from the client to the server when the document got closed in the client.
    async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.close_document(&uri).await;
        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack.did_close uri={}", uri.as_str()),
            )
            .await;
    }

    /// The [`workspace/didChangeWorkspaceFolders`] notification is sent from the client to the server to inform about workspace folder configuration changes.
    async fn did_change_workspace_folders(&self, params: lsp::DidChangeWorkspaceFoldersParams) {
        for folder in &params.event.added {
            let (workspace_handle, _) = self.insert_workspace(folder.uri.clone()).await;
            if let Err(error) = self.register_file_watch(workspace_handle.clone()).await {
                self.client
                    .log_message(
                        lsp::MessageType::ERROR,
                        format!(
                            "destack.did_change_workspace_folders.register_watch uri={} error={error}",
                            folder.uri.as_str()
                        ),
                    )
                    .await;
            }

            self.reindex_workspace(workspace_handle.clone()).await;
        }

        for folder in &params.event.removed {
            if let Some(workspace_handle) = self.remove_workspace(&folder.uri).await {
                if let Err(error) = self.unregister_file_watch(workspace_handle.clone()).await {
                    self.client
                        .log_message(
                            lsp::MessageType::ERROR,
                            format!(
                                "destack.did_change_workspace_folders.unregister_watch uri={} error={error}",
                                folder.uri.as_str()
                            ),
                        )
                        .await;
                }

                // update files in the workspace
                let new_uris = {
                    let mut workspace = workspace_handle.write().await;
                    let uris = workspace.document_uris();
                    for uri in &uris {
                        workspace.remove_document(uri);
                    }
                    uris
                };

                // reanalyze the workspace
                self.analyze_workspace(workspace_handle.clone(), Some(new_uris))
                    .await;

                self.client
                    .log_message(
                        lsp::MessageType::INFO,
                        format!(
                            "destack.did_change_workspace_folders.removed uri={}",
                            folder.uri.as_str()
                        ),
                    )
                    .await;
            }
        }
    }

    /// The [`workspace/didChangeWatchedFiles`] notification is sent from the client to the server when the client detects changes to files watched by the language client.
    async fn did_change_watched_files(&self, params: lsp::DidChangeWatchedFilesParams) {
        for change in params.changes {
            let Some(handle) = self.find_workspace_for_document(&change.uri).await else {
                continue;
            };

            let source_uri = lsp_uri_to_uri(&change.uri);
            let mut workspace = handle.write().await;
            if workspace.has_open_document(&source_uri) {
                continue; // do not override open documents
            }
            match change.typ {
                // sync the document if created/changed
                lsp::FileChangeType::CREATED | lsp::FileChangeType::CHANGED => {
                    match workspace.sync_document_from_disk(&source_uri) {
                        Ok(_) => {}
                        Err(error) => {
                            self.client
                                .log_message(
                                    lsp::MessageType::WARNING,
                                    format!(
                                        "destack.did_change_watched_files.sync_failed uri={} error={error}",
                                        change.uri.as_str()
                                    ),
                                )
                                .await;
                            continue;
                        }
                    }
                }
                // remove the document if deleted
                lsp::FileChangeType::DELETED => {
                    workspace.remove_document(&source_uri);
                }
                _ => {}
            }
            drop(workspace);

            self.analyze_workspace(handle.clone(), Some(vec![source_uri]))
                .await;

            self.client
                .log_message(
                    lsp::MessageType::INFO,
                    format!(
                        "destack.did_change_watched_files.change type={:?} uri={}",
                        change.typ,
                        change.uri.as_str()
                    ),
                )
                .await;
        }
    }

    /// The [`workspace/didCreateFiles`] notification is sent from the client to the server after files are created.
    async fn did_create_files(&self, params: lsp::CreateFilesParams) {
        for file in params.files {
            let Some(lsp_uri) = lsp::Uri::from_str(&file.uri).ok() else {
                self.client
                    .log_message(
                        lsp::MessageType::WARNING,
                        format!("destack.did_create_files.parse_failed uri={}", file.uri),
                    )
                    .await;
                continue;
            };

            // create file in workspace
            let workspace_handle = self.get_or_create_workspace_for_document(&lsp_uri).await;
            let mut workspace = workspace_handle.write().await;
            let uri = lsp_uri_to_uri(&lsp_uri);
            if workspace.has_open_document(&uri) {
                continue; // do not override open documents
            }
            match workspace.sync_document_from_disk(&uri) {
                Ok(_) => {}
                Err(error) => {
                    self.client
                        .log_message(
                            lsp::MessageType::WARNING,
                            format!(
                                "destack.did_create_files.sync_failed uri={} error={error}",
                                file.uri
                            ),
                        )
                        .await;
                    continue;
                }
            }
            drop(workspace);

            // reanalyze the workspace (partial)
            self.analyze_workspace(workspace_handle.clone(), Some(vec![uri]))
                .await;

            self.client
                .log_message(
                    lsp::MessageType::INFO,
                    format!("destack.did_create_files uri={}", lsp_uri.as_str()),
                )
                .await;
        }
    }

    /// The [`workspace/didRenameFiles`] notification is sent from the client to the server after files are renamed.
    async fn did_rename_files(&self, params: lsp::RenameFilesParams) {
        for rename in params.files {
            // convert uris
            let Some(old_uri) = lsp::Uri::from_str(&rename.old_uri).ok() else {
                self.client
                    .log_message(
                        lsp::MessageType::WARNING,
                        format!(
                            "destack.did_rename_files.parse_failed old_uri={}",
                            rename.old_uri
                        ),
                    )
                    .await;
                continue;
            };
            let Some(new_uri) = lsp::Uri::from_str(&rename.new_uri).ok() else {
                self.client
                    .log_message(
                        lsp::MessageType::WARNING,
                        format!(
                            "destack.did_rename_files.parse_failed new_uri={}",
                            rename.new_uri
                        ),
                    )
                    .await;
                continue;
            };

            // remove the old document
            if let Some(old_workspace_handle) = self.find_workspace_for_document(&old_uri).await {
                let old_uris = {
                    let mut workspace = old_workspace_handle.write().await;
                    let uri = lsp_uri_to_uri(&old_uri);
                    workspace.remove_document(&uri);
                    vec![uri]
                };
                self.analyze_workspace(old_workspace_handle.clone(), Some(old_uris))
                    .await;
            }

            // add the new document
            let new_workspace_handle = self.get_or_create_workspace_for_document(&new_uri).await;
            let mut workspace = new_workspace_handle.write().await;
            let uri = lsp_uri_to_uri(&new_uri);
            if workspace.has_open_document(&uri) {
                continue; // do not override open documents
            }
            match workspace.sync_document_from_disk(&uri) {
                Ok(_) => {}
                Err(error) => {
                    self.client
                        .log_message(
                            lsp::MessageType::WARNING,
                            format!(
                                "destack.did_rename_files.sync_failed new_uri={} error={}",
                                rename.new_uri, error
                            ),
                        )
                        .await;
                    return;
                }
            }
            self.analyze_workspace(new_workspace_handle.clone(), Some(vec![uri]))
                .await;

            self.client
                .log_message(
                    lsp::MessageType::INFO,
                    format!(
                        "destack.did_rename_files old_uri={} new_uri={}",
                        rename.old_uri, rename.new_uri
                    ),
                )
                .await;
        }
    }

    /// The [`workspace/didDeleteFiles`] notification is sent from the client to the server after files are deleted.
    async fn did_delete_files(&self, params: lsp::DeleteFilesParams) {
        for file in params.files {
            // parse the URI
            let Some(lsp_uri) = lsp::Uri::from_str(&file.uri).ok() else {
                self.client
                    .log_message(
                        lsp::MessageType::WARNING,
                        format!("destack.did_delete_files.parse_failed uri={}", file.uri),
                    )
                    .await;
                continue;
            };

            // remove the document
            if let Some(handle) = self.find_workspace_for_document(&lsp_uri).await {
                let uris = {
                    let mut workspace = handle.write().await;
                    let uri = lsp_uri_to_uri(&lsp_uri);
                    workspace.remove_document(&uri);
                    vec![uri]
                };

                self.analyze_workspace(handle.clone(), Some(uris)).await;

                self.client
                    .log_message(
                        lsp::MessageType::INFO,
                        format!("destack.did_delete_files uri={}", file.uri),
                    )
                    .await;
            }
        }
    }

    // ------------------------------------------------------------
    // Semantic Tokens
    // ------------------------------------------------------------

    /// The [`textDocument/semanticTokens/full`] request is sent from the client to the server to
    /// resolve the semantic tokens of a given file.
    async fn semantic_tokens_full(
        &self,
        params: lsp::SemanticTokensParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let Some(workspace_handle) = self.find_workspace_for_document(&uri).await else {
            return Ok(None);
        };
        let workspace = workspace_handle.read().await;
        let Some(semantic_tokens) = workspace.get_semantic_tokens_full(&lsp_uri_to_uri(&uri))
        else {
            return Ok(None);
        };
        let semantic_tokens = lsp::SemanticTokens {
            result_id: None,
            data: semantic_tokens,
        };

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack.semantic_tokens_full uri={}", uri.as_str()),
            )
            .await;
        Ok(Some(lsp::SemanticTokensResult::Tokens(semantic_tokens)))
    }

    /// The [`textDocument/semanticTokens/range`] request is sent from the client to the server to
    /// resolve the semantic tokens **for the visible range** of a given file.
    async fn semantic_tokens_range(
        &self,
        params: lsp::SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensRangeResult>> {
        let uri = params.text_document.uri;
        let Some(workspace_handle) = self.find_workspace_for_document(&uri).await else {
            return Ok(None);
        };
        let workspace = workspace_handle.read().await;
        let Some(semantic_tokens) =
            workspace.get_semantic_tokens_range(&lsp_uri_to_uri(&uri), &params.range)
        else {
            return Ok(None);
        };
        let semantic_tokens = lsp::SemanticTokens {
            result_id: None,
            data: semantic_tokens,
        };

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack.semantic_tokens_range uri={}", uri.as_str()),
            )
            .await;
        Ok(Some(lsp::SemanticTokensRangeResult::Tokens(
            semantic_tokens,
        )))
    }

    // ------------------------------------------------------------
    // Formatting
    // ------------------------------------------------------------

    /// The [`textDocument/formatting`] request is sent from the client to the server to
    /// format a given text document.
    async fn formatting(
        &self,
        params: lsp::DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        let uri = params.text_document.uri;
        let Some(workspace_handle) = self.find_workspace_for_document(&uri).await else {
            return Ok(None);
        };
        let workspace = workspace_handle.read().await;
        let Some(document) = workspace.get_document(&lsp_uri_to_uri(&uri)) else {
            return Ok(None);
        };
        let DocumentContent::Text { source, .. } = &document.content else {
            return Ok(None);
        };

        let Some(formatted) = workspace.format_document(document) else {
            return Ok(None);
        };
        let Some((end_line, end_character)) = source.get_position(source.len) else {
            return Ok(None);
        };
        let full_edit = lsp::TextEdit {
            range: lsp::Range {
                start: lsp::Position {
                    line: 0,
                    character: 0,
                },
                end: lsp::Position {
                    line: end_line,
                    character: end_character,
                },
            },
            new_text: formatted,
        };

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack.formatting uri={}", uri.as_str()),
            )
            .await;
        Ok(Some(vec![full_edit]))
    }
}
