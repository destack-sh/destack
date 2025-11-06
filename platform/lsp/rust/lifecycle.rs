use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use dyst_workspace::{FileContent, FileFile, Workspace};
use dyst_source::FileType;
use tokio::sync::RwLock;
use tower_lsp_server::{UriExt, jsonrpc, lsp_types as lsp};

use crate::DestackLanguageServer;
use crate::diagnostic::diagnostic_to_lsp_diagnostic;
use crate::workspace::{TRACKED_FORMATS, lsp_uri_to_uri, uri_to_lsp_uri};

impl DestackLanguageServer {
    /// Register file watchers for all existing workspaces.
    pub(crate) async fn register_all_file_watches(&self) {
        let workspace_handles: Vec<_> = {
            let guard = self.workspaces.read().await;
            guard.values().cloned().collect()
        };
        let num_workspaces = workspace_handles.len();
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

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack.register_all_watches workspaces={}", num_workspaces),
            )
            .await;
    }

    /// Register a file watcher for the given Workspace.
    pub(crate) async fn register_file_watch(
        &self,
        workspace_handle: Arc<RwLock<Workspace>>,
    ) -> jsonrpc::Result<()> {
        let workspace = workspace_handle.read().await;
        let mut workspace_watch_ids = self.workspace_watch_ids.write().await;
        let watch_registration_id = workspace_watch_ids
            .get(&workspace.root_uri.to_string())
            .cloned();
        if watch_registration_id.is_some() {
            return Ok(());
        }

        let registration_id = format!(
            "destack-watch-{}",
            self.next_watch_id.fetch_add(1, Ordering::Relaxed)
        );

        // track all supported source formats beneath the workspace root
        let base_uri = uri_to_lsp_uri(&workspace.root_uri);
        let watchers: Vec<lsp::FileSystemWatcher> = TRACKED_FORMATS
            .iter()
            .map(|format| lsp::FileSystemWatcher {
                glob_pattern: lsp::GlobPattern::Relative(lsp::RelativePattern {
                    base_uri: lsp::OneOf::Right(base_uri.clone()),
                    pattern: format.glob().to_string(),
                }),
                kind: Some(lsp::WatchKind::all()),
            })
            .collect();
        let options = lsp::DidChangeWatchedFilesRegistrationOptions { watchers };
        let register_options = serde_json::to_value(options)
            .map_err(|error| jsonrpc::Error::invalid_params(error.to_string()))?;
        let registration = lsp::Registration {
            id: registration_id.clone(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options: Some(register_options),
        };

        self.client.register_capability(vec![registration]).await?;
        workspace_watch_ids.insert(workspace.root_uri.to_string(), registration_id.clone());

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!(
                    "destack.register_file_watch registration_id={}",
                    registration_id
                ),
            )
            .await;

        Ok(())
    }

    /// Unregister the file watcher for the given Workspace.
    pub(crate) async fn unregister_file_watch(
        &self,
        workspace_handle: Arc<RwLock<Workspace>>,
    ) -> jsonrpc::Result<()> {
        let workspace = workspace_handle.read().await;
        let mut workspace_watch_ids = self.workspace_watch_ids.write().await;
        let Some(registration_id) = workspace_watch_ids
            .get(&workspace.root_uri.to_string())
            .cloned()
            .as_deref()
            .map(ToOwned::to_owned)
        else {
            return Ok(());
        };

        workspace_watch_ids.remove(&workspace.root_uri.to_string());
        self.client
            .unregister_capability(vec![lsp::Unregistration {
                id: registration_id,
                method: "workspace/didChangeWatchedFiles".to_string(),
            }])
            .await?;
        Ok(())
    }

    /// Get or create a Workspace for the given document URI.
    pub(crate) async fn get_or_create_workspace_for_document(
        &self,
        uri: &lsp::Uri,
    ) -> Arc<RwLock<Workspace>> {
        // bail if the workspace already exists
        if let Some(workspace_handle) = self.find_workspace_for_document(uri).await {
            return workspace_handle;
        }

        // insert workspace
        let created_root = self
            .derive_workspace_root(uri)
            .unwrap_or_else(|| uri.clone());
        let (workspace_handle, inserted) = self.insert_workspace(created_root).await;

        // index if it was newly inserted (race condition)
        if inserted {
            if let Err(error) = self.register_file_watch(workspace_handle.clone()).await {
                self.client
                    .log_message(
                        lsp::MessageType::ERROR,
                        format!(
                            "destack.get_or_create_workspace_for_document.register_watch error={error}"
                        ),
                    )
                    .await;
            }

            self.reindex_workspace(workspace_handle.clone()).await;
        }

        workspace_handle
    }

    /// Find the Workspace that contains the given document URI.
    pub(crate) async fn find_workspace_for_document(
        &self,
        uri: &lsp::Uri,
    ) -> Option<Arc<RwLock<Workspace>>> {
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
    pub(crate) async fn insert_workspace(&self, root: lsp::Uri) -> (Arc<RwLock<Workspace>>, bool) {
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
                let workspace = Workspace::empty(lsp_uri_to_uri(&root));
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
    pub(crate) async fn remove_workspace(&self, root: &lsp::Uri) -> Option<Arc<RwLock<Workspace>>> {
        let key = Self::get_workspace_key(root);
        let mut guard = self.workspaces.write().await;
        guard.remove(&key)
    }

    /// Generate a normalized key for workspace storage from URI.
    pub(crate) fn get_workspace_key(uri: &lsp::Uri) -> String {
        let mut value = uri.as_str().to_string();
        if !value.ends_with('/') {
            value.push('/');
        }
        value
    }

    /// Derive workspace root URI from document URI by going up one directory.
    pub(crate) fn derive_workspace_root(&self, uri: &lsp::Uri) -> Option<lsp::Uri> {
        let path = uri.to_file_path()?.into_owned();
        let mut path = path;
        if !path.pop() {
            return None;
        }
        lsp::Uri::from_file_path(&path)
    }

    /// Collect diagnostics for a URI and encode them for the client.
    pub(crate) fn get_diagnostics_for_uri(
        workspace: &Workspace,
        uri: &dyst_source::Uri,
    ) -> Vec<lsp::Diagnostic> {
        let Some(document) = workspace.get_file(uri) else {
            return Vec::new();
        };
        let FileContent::File(FileFile { source, .. }) = &document.content else {
            return Vec::new();
        };
        workspace
            .get_diagnostics_for_source(source.id)
            .into_iter()
            .map(|diagnostic| diagnostic_to_lsp_diagnostic(&diagnostic, source))
            .collect()
    }

    /// Update an open document's content in the appropriate Workspace.
    pub(crate) async fn upsert_open_text_document(
        &self,
        lsp_uri: &lsp::Uri,
        format: FileType,
        content: String,
    ) {
        // update the document
        let workspace_handle = self.get_or_create_workspace_for_document(lsp_uri).await;
        let mut workspace = workspace_handle.write().await;
        let uri = lsp_uri_to_uri(lsp_uri);
        workspace.upsert_text_file(&uri, format, true, content);
        drop(workspace);

        // re-analyze the workspace
        self.trigger_analyze_workspace(workspace_handle.clone(), Some(vec![uri]))
            .await;

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack.upsert_open_text_document uri={}", lsp_uri.as_str(),),
            )
            .await;
    }

    /// Trigger re-analyze (part of) a workspace.
    /// All relevant packages will be re-analyzed.
    pub(crate) async fn trigger_analyze_workspace(
        &self,
        workspace_handle: Arc<RwLock<Workspace>>,
        uris: Option<Vec<dyst_source::Uri>>,
    ) {
        // publish new AST diagnostics immediately
        let workspace = workspace_handle.read().await;
        let uris = uris.unwrap_or_else(|| workspace.files_uris());
        for uri in &uris {
            let lsp_uri = uri_to_lsp_uri(uri);
            let diagnostics = Self::get_diagnostics_for_uri(&workspace, uri);
            self.client
                .publish_diagnostics(lsp_uri, diagnostics, None)
                .await;
        }

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!(
                    "destack.analyze_workspace uris={} diagnostics={}",
                    uris.len(),
                    workspace.diagnostics().len()
                ),
            )
            .await;

        // re-analyze in background (for DIR-level stuff)
        // TODO #Incomplete: re-analyze/compile in background
    }

    /// Refresh a workspace from disk and publish updated diagnostics.
    pub(crate) async fn reindex_workspace(&self, workspace_handle: Arc<RwLock<Workspace>>) {
        let mut workspace = workspace_handle.write().await;
        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!(
                    "destack.reindex_workspace.start root={}",
                    workspace.root_uri
                ),
            )
            .await;

        // reindex the workspace
        let uris = match workspace.reload_from_disk() {
            Ok(outcome) => {
                // merge updates and removals so diagnostics clear for former files
                let mut combined = outcome.updated;
                combined.extend(outcome.removed);
                combined
            }
            Err(error) => {
                let root_display = workspace.root_uri.as_ref().to_string();
                self.client
                    .log_message(
                        lsp::MessageType::ERROR,
                        format!("destack.reindex_workspace root={root_display} error={error}"),
                    )
                    .await;
                return;
            }
        };
        let root = workspace.root_uri.clone();
        let num_diagnostics = workspace.diagnostics().len();
        drop(workspace);

        let num_uris = uris.len();
        self.trigger_analyze_workspace(workspace_handle.clone(), Some(uris))
            .await;

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!(
                    "destack.reindex_workspace root={} uris={} diagnostics={}",
                    root, num_diagnostics, num_uris
                ),
            )
            .await;
    }

    /// Index every known workspace.
    pub(crate) async fn reindex_all_workspaces(&self) {
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
                    "destack.index_all_workspaces workspaces={}",
                    workspace_handles.len()
                ),
            )
            .await;
    }

    /// Remove document from its Workspace.
    pub(crate) async fn close_document(&self, lsp_uri: &lsp::Uri) {
        let Some(workspace_handle) = self.find_workspace_for_document(lsp_uri).await else {
            return;
        };

        let mut workspace = workspace_handle.write().await;
        let uri = lsp_uri_to_uri(lsp_uri);

        // resync the document
        match workspace.reload_file_from_disk(&uri) {
            Ok(_) => {
                let diagnostics = Self::get_diagnostics_for_uri(&workspace, &uri);
                self.client
                    .publish_diagnostics(lsp_uri.clone(), diagnostics, None)
                    .await;
            }
            Err(error) => {
                self.client
                    .log_message(
                        lsp::MessageType::WARNING,
                        format!(
                            "destack.close_document.sync_failed uri={} error={error}",
                            lsp_uri.as_str()
                        ),
                    )
                    .await;
                return;
            }
        }

        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack.close_document uri={}", lsp_uri.as_str()),
            )
            .await;
    }

    /// Add initial workspaces from initialization parameters.
    pub(crate) async fn add_initial_workspaces(&self, params: &lsp::InitializeParams) {
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
