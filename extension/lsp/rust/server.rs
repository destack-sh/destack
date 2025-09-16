//! Async language server implementation built on tower-lsp-server.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::RwLock;
use tower_lsp_server::{Client, LanguageServer, UriExt, jsonrpc, lsp_types as lsp};

use crate::semantic;
use crate::workspace::Workspace;

#[derive(Debug)]
pub struct DestackLanguageServer {
    client: Client,
    workspaces: RwLock<HashMap<String, Arc<RwLock<Workspace>>>>,
    next_watch_id: AtomicU64,
}

impl DestackLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            workspaces: RwLock::new(HashMap::new()),
            next_watch_id: AtomicU64::new(0),
        }
    }

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
                        format!("destack: failed to register watcher: {error}"),
                    )
                    .await;
            }
        }
    }

    async fn register_watch(
        &self,
        workspace_handle: Arc<RwLock<Workspace>>,
    ) -> jsonrpc::Result<()> {
        let mut workspace = workspace_handle.write().await;
        if workspace.watch_registration_id().is_some() {
            return Ok(());
        }

        let registration_id = format!(
            "destack-watch-{}",
            self.next_watch_id.fetch_add(1, Ordering::Relaxed)
        );

        let pattern = lsp::RelativePattern {
            base_uri: lsp::OneOf::Right(workspace.root_uri().clone()),
            pattern: "**/*.ds".to_string(),
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

        workspace.set_watch_registration_id(registration_id);
        Ok(())
    }

    async fn unregister_watch(
        &self,
        workspace_handle: Arc<RwLock<Workspace>>,
    ) -> jsonrpc::Result<()> {
        let mut workspace = workspace_handle.write().await;
        let Some(registration_id) = workspace.watch_registration_id().map(ToOwned::to_owned) else {
            return Ok(());
        };

        workspace.clear_watch_registration_id();
        self.client
            .unregister_capability(vec![lsp::Unregistration {
                id: registration_id,
                method: "workspace/didChangeWatchedFiles".to_string(),
            }])
            .await?;
        Ok(())
    }

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
                    format!("destack: failed to register workspace watch: {error}"),
                )
                .await;
        }
        handle
    }

    async fn find_workspace_for_document(&self, uri: &lsp::Uri) -> Option<Arc<RwLock<Workspace>>> {
        let key = uri.as_str();
        let guard = self.workspaces.read().await;
        guard
            .iter()
            .filter(|(root, _)| key.starts_with(root.as_str()))
            .max_by_key(|(root, _)| root.len())
            .map(|(_, handle)| handle.clone())
    }

    async fn insert_workspace(&self, root_uri: lsp::Uri) -> (Arc<RwLock<Workspace>>, bool) {
        let key = Self::workspace_key(&root_uri);
        {
            let guard = self.workspaces.read().await;
            if let Some(existing) = guard.get(&key) {
                return (existing.clone(), false);
            }
        }

        let mut guard = self.workspaces.write().await;
        match guard.entry(key) {
            Entry::Vacant(vacant) => {
                let handle = Arc::new(RwLock::new(Workspace::new(root_uri)));
                let cloned = handle.clone();
                vacant.insert(handle);
                (cloned, true)
            }
            Entry::Occupied(occupied) => (occupied.get().clone(), false),
        }
    }

    async fn remove_workspace(&self, root_uri: &lsp::Uri) -> Option<Arc<RwLock<Workspace>>> {
        let key = Self::workspace_key(root_uri);
        let mut guard = self.workspaces.write().await;
        guard.remove(&key)
    }

    fn workspace_key(uri: &lsp::Uri) -> String {
        let mut value = uri.as_str().to_string();
        if !value.ends_with('/') {
            value.push('/');
        }
        value
    }

    fn derive_workspace_root(&self, uri: &lsp::Uri) -> Option<lsp::Uri> {
        let path = uri.to_file_path()?.into_owned();
        let mut path = path;
        if !path.pop() {
            return None;
        }
        lsp::Uri::from_file_path(&path)
    }

    async fn update_document(&self, uri: &lsp::Uri, content: String) {
        let handle = self.ensure_workspace_for_document(uri).await;
        let mut workspace = handle.write().await;
        workspace.upsert_document(uri, content);
    }

    async fn close_document(&self, uri: &lsp::Uri) {
        if let Some(handle) = self.find_workspace_for_document(uri).await {
            let mut workspace = handle.write().await;
            workspace.remove_document(uri);
        }
    }

    async fn compute_semantic_tokens(
        &self,
        uri: &lsp::Uri,
        range: Option<lsp::Range>,
    ) -> Option<lsp::SemanticTokens> {
        let handle = self.find_workspace_for_document(uri).await?;
        let workspace = handle.read().await;
        let encoded = match range {
            Some(range) => workspace.semantic_tokens_range(uri, &range),
            None => workspace.semantic_tokens_full(uri),
        }?;
        Some(lsp::SemanticTokens {
            result_id: None,
            data: encoded,
        })
    }

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
    async fn initialize(
        &self,
        params: lsp::InitializeParams,
    ) -> jsonrpc::Result<lsp::InitializeResult> {
        self.add_initial_workspaces(&params).await;

        let capabilities = lsp::ServerCapabilities {
            text_document_sync: Some(lsp::TextDocumentSyncCapability::Kind(
                lsp::TextDocumentSyncKind::FULL,
            )),
            hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
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
                file_operations: None,
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

    async fn initialized(&self, _: lsp::InitializedParams) {
        self.client
            .log_message(lsp::MessageType::INFO, "destack: initialized")
            .await;
        self.register_all_watches().await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        self.client
            .log_message(lsp::MessageType::INFO, "destack: shutdown")
            .await;
        Ok(())
    }

    async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        self.update_document(&uri, params.text_document.text).await;
        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack: did_open {}", uri.as_str()),
            )
            .await;
    }

    async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.update_document(&uri, change.text).await;
        }
        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack: did_change {}", uri.as_str()),
            )
            .await;
    }

    async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.close_document(&uri).await;
        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack: did_close {}", uri.as_str()),
            )
            .await;
    }

    async fn did_change_workspace_folders(&self, params: lsp::DidChangeWorkspaceFoldersParams) {
        for folder in params.event.added {
            let (handle, inserted) = self.insert_workspace(folder.uri.clone()).await;
            if inserted && let Err(error) = self.register_watch(handle).await {
                self.client
                    .log_message(
                        lsp::MessageType::ERROR,
                        format!("destack: failed to register workspace watch: {error}"),
                    )
                    .await;
            }
        }

        for folder in params.event.removed {
            if let Some(handle) = self.remove_workspace(&folder.uri).await
                && let Err(error) = self.unregister_watch(handle).await
            {
                self.client
                    .log_message(
                        lsp::MessageType::ERROR,
                        format!(
                            "destack: failed to unregister workspace watch for {}: {error}",
                            folder.uri.as_str()
                        ),
                    )
                    .await;
            }
        }
    }

    async fn did_change_watched_files(&self, params: lsp::DidChangeWatchedFilesParams) {
        for change in params.changes {
            self.client
                .log_message(
                    lsp::MessageType::INFO,
                    format!(
                        "destack: file change {:?} for {}",
                        change.typ,
                        change.uri.as_str()
                    ),
                )
                .await;
        }
    }

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
