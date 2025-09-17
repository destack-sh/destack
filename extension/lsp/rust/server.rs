//! Async language server implementation built on tower-lsp-server.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::RwLock;
use tower_lsp_server::{Client, LanguageServer, UriExt, jsonrpc, lsp_types as lsp};

use crate::diagnostic::diagnostic_to_lsp_diagnostic;
use crate::semantic;
use crate::workspace::{Workspace, lsp_uri_to_uri, uri_to_lsp_uri};

pub const DYST_FILE_GLOB: &str = "**/*.ds";
pub const DYST_FILE_EXTENSION: &str = "ds";

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
    async fn register_all_file_watches(&self) {
        let workspace_handles: Vec<_> = {
            let guard = self.workspaces.read().await;
            guard.values().cloned().collect()
        };

        for workspace_handle in workspace_handles {
            if let Err(error) = self.register_file_watch(workspace_handle.clone()).await {
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
    async fn register_file_watch(
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
    async fn unregister_file_watch(
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
        if let Some(workspace_handle) = self.find_workspace_for_document(uri).await {
            return workspace_handle;
        }

        let created_root = self
            .derive_workspace_root(uri)
            .unwrap_or_else(|| uri.clone());
        let (workspace_handle, inserted) = self.insert_workspace(created_root).await;
        if inserted {
            if let Err(error) = self.register_file_watch(workspace_handle.clone()).await {
                self.client
                    .log_message(
                        lsp::MessageType::ERROR,
                        format!(
                            "destack.ensure_workspace_for_document.register_watch error={error}"
                        ),
                    )
                    .await;
            }

            self.reindex_workspace(workspace_handle.clone()).await;
        }
        workspace_handle
    }

    /// Find the Workspace that contains the given document URI.
    async fn find_workspace_for_document(&self, uri: &lsp::Uri) -> Option<Arc<RwLock<Workspace>>> {
        let key = uri.as_str();
        let guard = self.workspaces.read().await;
        guard
            .iter()
            .filter(|(root, _)| key.starts_with(root.as_str()))
            .max_by_key(|(root, _)| root.len())
            .map(|(_, workspace_handle)| workspace_handle.clone())
    }

    /// Insert a new Workspace or get existing one for the given root URI.
    /// Returns the Workspace handle and whether it was newly inserted.
    async fn insert_workspace(&self, root: lsp::Uri) -> (Arc<RwLock<Workspace>>, bool) {
        let key = Self::get_workspace_key(&root);
        {
            let workspace = self.workspaces.read().await;
            if let Some(existing) = workspace.get(&key) {
                return (existing.clone(), false);
            }
        }

        let mut guard = self.workspaces.write().await;
        let (workspace_handle, inserted) = match guard.entry(key.clone()) {
            Entry::Vacant(vacant) => {
                let workspace = Workspace::new(lsp_uri_to_uri(&root));
                let workspace_handle = Arc::new(RwLock::new(workspace));
                vacant.insert(workspace_handle.clone());
                (workspace_handle, true)
            }
            Entry::Occupied(occupied) => (occupied.get().clone(), false),
        };

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack.workspace.insert key={key}"),
            )
            .await;

        (workspace_handle, inserted)
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

    /// Collect diagnostics for a URI and encode them for the client.
    fn diagnostics_for_uri(
        workspace: &Workspace,
        uri: &dyst_language_source::Uri,
    ) -> Vec<lsp::Diagnostic> {
        workspace
            .get_document(uri)
            .map(|document| {
                workspace
                    .get_diagnostics(Some(document.source.id))
                    .into_iter()
                    .map(|diagnostic| diagnostic_to_lsp_diagnostic(&diagnostic, &document.source))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Update an open document's content in the appropriate Workspace.
    async fn update_open_document(&self, lsp_uri: &lsp::Uri, content: String) {
        let workspace_handle = self.ensure_workspace_for_document(lsp_uri).await;
        let (uri, diagnostics) = {
            let mut workspace = workspace_handle.write().await;
            let uri = lsp_uri_to_uri(lsp_uri);
            workspace.upsert_document(&uri, content, true);
            let diagnostics = Self::diagnostics_for_uri(&workspace, &uri);
            (uri_to_lsp_uri(&uri), diagnostics)
        };

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    /// Reanalyze a batch of URIs.
    async fn reanalyze_workspace(
        &self,
        workspace_handle: Arc<RwLock<Workspace>>,
        uris: Vec<dyst_language_source::Uri>,
    ) {
        if uris.is_empty() {
            return;
        }

        // todo! analyze in background?

        // read the workspace once
        let workspace = workspace_handle.read().await;
        for uri in uris {
            let lsp_uri = uri_to_lsp_uri(&uri);
            let diagnostics = Self::diagnostics_for_uri(&workspace, &uri);
            self.client
                .publish_diagnostics(lsp_uri, diagnostics, None)
                .await;
        }

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!(
                    "destack.reanalyze_workspace diagnostics={}",
                    workspace.diagnostics().len()
                ),
            )
            .await;
    }

    /// Refresh a workspace from disk and publish updated diagnostics.
    async fn reindex_workspace(&self, workspace_handle: Arc<RwLock<Workspace>>) {
        let mut workspace = workspace_handle.write().await;
        let uris = match workspace.reindex_from_disk() {
            Ok(outcome) => {
                // merge updates and removals so diagnostics clear for former files
                let mut combined = outcome.updated;
                combined.extend(outcome.removed);
                combined
            }
            Err(error) => {
                let root_display = workspace.root.as_ref().to_string();
                self.client
                    .log_message(
                        lsp::MessageType::ERROR,
                        format!("destack.reindex_workspace root={root_display} error={error}"),
                    )
                    .await;
                return;
            }
        };
        let num_uris = uris.len();
        self.reanalyze_workspace(workspace_handle.clone(), uris)
            .await;

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!(
                    "destack.reindex_workspace diagnostics={} uris={}",
                    workspace.diagnostics().len(),
                    num_uris
                ),
            )
            .await;
    }

    /// Reindex every known workspace.
    async fn reindex_all_workspaces(&self) {
        let workspace_handles: Vec<_> = {
            let guard = self.workspaces.read().await;
            guard.values().cloned().collect()
        };

        for workspace_handle in &workspace_handles {
            self.reindex_workspace(workspace_handle.clone()).await;
        }

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!(
                    "destack.reindex_all_workspaces workspaces={}",
                    workspace_handles.len()
                ),
            )
            .await;
    }

    /// Remove document from its Workspace.
    async fn close_document(&self, uri: &lsp::Uri) {
        let Some(workspace_handle) = self.find_workspace_for_document(uri).await else {
            return;
        };

        let mut workspace = workspace_handle.write().await;
        let internal_uri = lsp_uri_to_uri(uri);
        workspace.set_document_is_open(&internal_uri, false);

        // resync the document
        match workspace.sync_document_from_disk(&internal_uri) {
            Ok(_) => {
                let diagnostics = Self::diagnostics_for_uri(&workspace, &internal_uri);
                self.client
                    .publish_diagnostics(uri.clone(), diagnostics, None)
                    .await;
            }
            Err(error) => {
                workspace.remove_document(&internal_uri);
                self.client
                    .log_message(
                        lsp::MessageType::WARNING,
                        format!(
                            "destack.close_document.sync_failed uri={} error={error}",
                            uri.as_str()
                        ),
                    )
                    .await;
                return;
            }
        }

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack.close_document uri={}", uri.as_str()),
            )
            .await;
    }

    /// Compute semantic tokens for a document, optionally restricted to a range.
    async fn compute_semantic_tokens(
        &self,
        uri: &lsp::Uri,
        range: Option<lsp::Range>,
    ) -> Option<lsp::SemanticTokens> {
        let workspace_handle = self.find_workspace_for_document(uri).await?;
        let workspace = workspace_handle.read().await;
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
        let Some(folders) = &params.workspace_folders else {
            return;
        };

        // insert workspaces
        let mut workspace_handles = Vec::with_capacity(folders.len());
        for folder in folders {
            let (workspace_handle, _) = self.insert_workspace(folder.uri.clone()).await;
            workspace_handles.push(workspace_handle);
        }

        // reindex workspaces
        for workspace_handle in workspace_handles {
            self.reindex_workspace(workspace_handle.clone()).await;
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
                    will_create: None,
                    will_rename: None,
                    will_delete: None,
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
        self.register_all_file_watches().await;
        self.reindex_all_workspaces().await;
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
        self.update_open_document(&uri, params.text_document.text)
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
            self.update_open_document(&uri, change.text).await;
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
            let (workspace_handle, _) = self.insert_workspace(folder.uri.clone()).await;
            if let Err(error) = self.register_file_watch(workspace_handle.clone()).await {
                self.client
                    .log_message(
                        lsp::MessageType::ERROR,
                        format!(
                            "destack.did_change_workspace_folders.register_watch error={error}"
                        ),
                    )
                    .await;
            }

            self.reindex_workspace(workspace_handle.clone()).await;
        }

        for folder in params.event.removed {
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
                self.reanalyze_workspace(workspace_handle.clone(), new_uris)
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

            self.reanalyze_workspace(handle.clone(), vec![source_uri])
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
            let workspace_handle = self.ensure_workspace_for_document(&lsp_uri).await;
            let mut workspace = workspace_handle.write().await;
            let uri = lsp_uri_to_uri(&lsp_uri);
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
            self.reanalyze_workspace(workspace_handle.clone(), vec![uri])
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
                self.reanalyze_workspace(old_workspace_handle.clone(), old_uris)
                    .await;
            }

            // add the new document
            let new_workspace_handle = self.ensure_workspace_for_document(&new_uri).await;
            let mut workspace = new_workspace_handle.write().await;
            let uri = lsp_uri_to_uri(&new_uri);
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
            self.reanalyze_workspace(new_workspace_handle.clone(), vec![uri])
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

                self.reanalyze_workspace(handle.clone(), uris).await;

                self.client
                    .log_message(
                        lsp::MessageType::INFO,
                        format!("destack.did_delete_files uri={}", file.uri),
                    )
                    .await;
            }
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
