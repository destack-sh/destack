//! Async language server implementation built on tower-lsp-server.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::RwLock;
use tower_lsp_server::{Client, LanguageServer, UriExt, jsonrpc, lsp_types as lsp};

use crate::diagnostic::diagnostic_to_lsp_diagnostic;
use crate::semantic;
use crate::workspace::{Workspace, lsp_uri_to_uri, uri_to_lsp_uri};

const DYST_FILE_GLOB: &str = "**/*.ds";

#[derive(Debug)]
pub struct DestackLanguageServer {
    /// The client that the language server is connected to.
    client: Client,
    /// The workspaces that the language server is connected to.
    workspaces: RwLock<HashMap<String, Arc<RwLock<Workspace>>>>,
    /// The next watch id to use for the language server.
    next_watch_id: AtomicU64,
}

impl DestackLanguageServer {
    /// Create a new language server instance with the given client.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            workspaces: RwLock::new(HashMap::new()),
            next_watch_id: AtomicU64::new(0),
        }
    }

    /// Register file watchers for all existing workspaces.
    async fn register_all_watches(&self) {
        let handles: Vec<_> = {
            let guard = self.workspaces.read().await;
            guard.values().cloned().collect()
        };

        for handle in handles {
            if let Err(error) = self.register_watch(handle.clone()).await {
                self.client
                    .log_message(
                        lsp::MessageType::ERROR,
                        format!("destack.register_all_watches.register_watch error={error}"),
                    )
                    .await;
            }
        }
    }

    /// Register a file watcher for the given Workspace.
    async fn register_watch(
        &self,
        workspace_handle: Arc<RwLock<Workspace>>,
    ) -> jsonrpc::Result<()> {
        let mut workspace = workspace_handle.write().await;
        if workspace.watch_registration_id.is_some() {
            return Ok(());
        }

        let registration_id = format!(
            "destack-watch-{}",
            self.next_watch_id.fetch_add(1, Ordering::Relaxed)
        );

        // create file watcher for .ds files in workspace
        let pattern = lsp::RelativePattern {
            base_uri: lsp::OneOf::Right(uri_to_lsp_uri(&workspace.root)),
            pattern: DYST_FILE_GLOB.to_string(),
        };
        let watchers = vec![lsp::FileSystemWatcher {
            glob_pattern: lsp::GlobPattern::Relative(pattern),
            kind: Some(lsp::WatchKind::all()),
        }];
        let options = lsp::DidChangeWatchedFilesRegistrationOptions { watchers };
        let register_options = serde_json::to_value(options)
            .map_err(|error| jsonrpc::Error::invalid_params(error.to_string()))?;
        let registration = lsp::Registration {
            id: registration_id.clone(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options: Some(register_options),
        };

        self.client.register_capability(vec![registration]).await?;

        workspace.watch_registration_id = Some(registration_id);
        Ok(())
    }

    /// Unregister the file watcher for the given Workspace.
    async fn unregister_watch(
        &self,
        workspace_handle: Arc<RwLock<Workspace>>,
    ) -> jsonrpc::Result<()> {
        let mut workspace = workspace_handle.write().await;
        let Some(registration_id) = workspace
            .watch_registration_id
            .clone()
            .as_deref()
            .map(ToOwned::to_owned)
        else {
            return Ok(());
        };

        workspace.watch_registration_id = None;
        self.client
            .unregister_capability(vec![lsp::Unregistration {
                id: registration_id,
                method: "workspace/didChangeWatchedFiles".to_string(),
            }])
            .await?;
        Ok(())
    }

    /// Get or create a Workspace for the given document URI.
    async fn ensure_workspace_for_document(&self, uri: &lsp::Uri) -> Arc<RwLock<Workspace>> {
        if let Some(handle) = self.find_workspace_for_document(uri).await {
            return handle;
        }

        let created_root = self
            .derive_workspace_root(uri)
            .unwrap_or_else(|| uri.clone());
        let (handle, inserted) = self.insert_workspace(created_root).await;
        if inserted && let Err(error) = self.register_watch(handle.clone()).await {
            self.client
                .log_message(
                    lsp::MessageType::ERROR,
                    format!("destack.ensure_workspace_for_document.register_watch error={error}"),
                )
                .await;
        }
        handle
    }

    /// Find the Workspace that contains the given document URI.
    async fn find_workspace_for_document(&self, uri: &lsp::Uri) -> Option<Arc<RwLock<Workspace>>> {
        let key = uri.as_str();
        let guard = self.workspaces.read().await;
        guard
            .iter()
            .filter(|(root, _)| key.starts_with(root.as_str()))
            .max_by_key(|(root, _)| root.len())
            .map(|(_, handle)| handle.clone())
    }

    /// Insert a new Workspace or get existing one for the given root URI.
    /// Returns the Workspace handle and whether it was newly inserted.
    async fn insert_workspace(&self, root: lsp::Uri) -> (Arc<RwLock<Workspace>>, bool) {
        let key = Self::get_workspace_key(&root);
        {
            let guard = self.workspaces.read().await;
            if let Some(existing) = guard.get(&key) {
                return (existing.clone(), false);
            }
        }

        let mut guard = self.workspaces.write().await;
        let (handle, inserted) = match guard.entry(key.clone()) {
            Entry::Vacant(vacant) => {
                let workspace = Workspace::new(lsp_uri_to_uri(&root));
                let handle = Arc::new(RwLock::new(workspace));
                let cloned = handle.clone();
                vacant.insert(handle);
                (cloned, true)
            }
            Entry::Occupied(occupied) => (occupied.get().clone(), false),
        };

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack.workspace.insert key={key}"),
            )
            .await;

        (handle, inserted)
    }

    /// Remove and return the Workspace for the given root URI.
    async fn remove_workspace(&self, root: &lsp::Uri) -> Option<Arc<RwLock<Workspace>>> {
        let key = Self::get_workspace_key(root);
        let mut guard = self.workspaces.write().await;
        guard.remove(&key)
    }

    /// Generate a normalized key for workspace storage from URI.
    fn get_workspace_key(uri: &lsp::Uri) -> String {
        let mut value = uri.as_str().to_string();
        if !value.ends_with('/') {
            value.push('/');
        }
        value
    }

    /// Derive workspace root URI from document URI by going up one directory.
    fn derive_workspace_root(&self, uri: &lsp::Uri) -> Option<lsp::Uri> {
        let path = uri.to_file_path()?.into_owned();
        let mut path = path;
        if !path.pop() {
            return None;
        }
        lsp::Uri::from_file_path(&path)
    }

    /// Update document content in the appropriate Workspace.
    async fn update_document(&self, lsp_uri: &lsp::Uri, content: String) {
        let handle = self.ensure_workspace_for_document(lsp_uri).await;
        let mut workspace = handle.write().await;
        let uri = lsp_uri_to_uri(lsp_uri);
        workspace.upsert_document(&uri, content);
        let document = workspace.get_document(&uri).unwrap();

        // todo! schedule analysis properly?

        let diagnostics = workspace
            .get_diagnostics(None)
            .iter()
            .map(|diagnostic| diagnostic_to_lsp_diagnostic(diagnostic, &document.source))
            .collect();
        self.client
            .publish_diagnostics(lsp_uri.clone(), diagnostics, None)
            .await;
    }

    /// Remove document from its Workspace.
    async fn close_document(&self, uri: &lsp::Uri) {
        if let Some(handle) = self.find_workspace_for_document(uri).await {
            let mut workspace = handle.write().await;
            workspace.remove_document(&lsp_uri_to_uri(uri));
        }
    }

    /// Compute semantic tokens for a document, optionally restricted to a range.
    async fn compute_semantic_tokens(
        &self,
        uri: &lsp::Uri,
        range: Option<lsp::Range>,
    ) -> Option<lsp::SemanticTokens> {
        let handle = self.find_workspace_for_document(uri).await?;
        let workspace = handle.read().await;
        let encoded = match range {
            Some(range) => workspace.semantic_tokens_range(&lsp_uri_to_uri(uri), &range),
            None => workspace.semantic_tokens_full(&lsp_uri_to_uri(uri)),
        }?;
        Some(lsp::SemanticTokens {
            result_id: None,
            data: encoded,
        })
    }

    /// Add initial workspaces from initialization parameters.
    async fn add_initial_workspaces(&self, params: &lsp::InitializeParams) {
        if let Some(folders) = &params.workspace_folders {
            for folder in folders {
                self.insert_workspace(folder.uri.clone()).await;
            }
            return;
        }

        #[allow(deprecated)]
        if let Some(uri) = &params.root_uri {
            self.insert_workspace(uri.clone()).await;
        } else {
            #[allow(deprecated)]
            if let Some(path) = &params.root_path {
                if let Ok(uri) = lsp::Uri::from_str(path) {
                    self.insert_workspace(uri).await;
                } else if !path.is_empty() {
                    let path_buf = PathBuf::from(path);
                    if let Some(uri) = lsp::Uri::from_file_path(path_buf) {
                        self.insert_workspace(uri).await;
                    }
                }
            }
        }
    }
}

impl LanguageServer for DestackLanguageServer {
    /// The [`initialize`] request is the first request sent from the client to the server.
    async fn initialize(
        &self,
        params: lsp::InitializeParams,
    ) -> jsonrpc::Result<lsp::InitializeResult> {
        self.add_initial_workspaces(&params).await;

        let file_operation_filters = vec![lsp::FileOperationFilter {
            scheme: None,
            pattern: lsp::FileOperationPattern {
                glob: DYST_FILE_GLOB.to_string(),
                matches: Some(lsp::FileOperationPatternKind::File),
                options: Some(lsp::FileOperationPatternOptions {
                    ignore_case: Some(true),
                }),
            },
        }];

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
                    will_create: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    will_rename: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    will_delete: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                }),
            }),
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

    /// The [`initialized`] notification is sent from the client to the server after the client received the result of the initialize request but before the client sends anything else.
    async fn initialized(&self, _: lsp::InitializedParams) {
        self.client
            .log_message(lsp::MessageType::INFO, "destack.initialized")
            .await;
        self.register_all_watches().await;
    }

    /// The [`shutdown`] request asks the server to gracefully shut down, but to not exit.
    async fn shutdown(&self) -> jsonrpc::Result<()> {
        self.client
            .log_message(lsp::MessageType::INFO, "destack.shutdown")
            .await;
        Ok(())
    }

    /// The [`textDocument/didOpen`] notification is sent from the client to the server to signal that a new text document has been opened by the client.
    async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        self.update_document(&uri, params.text_document.text).await;
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
            self.update_document(&uri, change.text).await;
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
        for folder in params.event.added {
            let (handle, inserted) = self.insert_workspace(folder.uri.clone()).await;
            if inserted && let Err(error) = self.register_watch(handle).await {
                self.client
                    .log_message(
                        lsp::MessageType::ERROR,
                        format!(
                            "destack.did_change_workspace_folders.register_watch error={error}"
                        ),
                    )
                    .await;
            }
        }

        for folder in params.event.removed {
            if let Some(handle) = self.remove_workspace(&folder.uri).await
                && let Err(_) = self.unregister_watch(handle).await
            {
                self.client
                    .log_message(
                        lsp::MessageType::ERROR,
                        format!(
                            "destack.did_change_workspace_folders.unregister_watch uri={}",
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
            let uri = lsp_uri_to_uri(&change.uri);
            if change.typ == lsp::FileChangeType::CREATED
                || change.typ == lsp::FileChangeType::CHANGED
            {
                // todo!
            } else if change.typ == lsp::FileChangeType::DELETED {
                self.close_document(&uri_to_lsp_uri(&uri)).await;
            }
            self.client
                .log_message(
                    lsp::MessageType::INFO,
                    format!(
                        "destack.did_change_watched_files.change type={:?} uri={}",
                        change.typ,
                        uri.as_ref()
                    ),
                )
                .await;
        }
    }

    /// The [`textDocument/semanticTokens/full`] request is sent from the client to the server to
    /// resolve the semantic tokens of a given file.
    async fn semantic_tokens_full(
        &self,
        params: lsp::SemanticTokensParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensResult>> {
        match self
            .compute_semantic_tokens(&params.text_document.uri, None)
            .await
        {
            Some(tokens) => Ok(Some(lsp::SemanticTokensResult::Tokens(tokens))),
            None => Ok(None),
        }
    }

    /// The [`textDocument/semanticTokens/range`] request is sent from the client to the server to
    /// resolve the semantic tokens **for the visible range** of a given file.
    async fn semantic_tokens_range(
        &self,
        params: lsp::SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensRangeResult>> {
        match self
            .compute_semantic_tokens(&params.text_document.uri, Some(params.range))
            .await
        {
            Some(tokens) => Ok(Some(lsp::SemanticTokensRangeResult::Tokens(tokens))),
            None => Ok(None),
        }
    }
}
