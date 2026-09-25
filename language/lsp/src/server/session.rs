use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::from_value;
use tspp_lsp_server::{Client, UriExt, jsonrpc};
use tspp_lsp_types as lsp;
use tspp_query as query;
use tspp_repository::{Host, Revision, SourceRoot, Trace};
use tspp_session::Executor;
use tspp_source::{File, TextChange};
use tspp_workspace::{QueryFile, Workspace};

use super::{Project, ProjectSet, TSPP_URI_SCHEME, internal_error};
use crate::query::DiagnosticDelivery;

/// State installed after one language server initialization.
#[derive(Debug)]
pub(super) struct ServerSession {
    /// Artifact executor shared by every project session.
    executor: Arc<Executor>,
    /// Repository capabilities shared by every project.
    host: Host,
    /// Projects discovered for the current editor workspace.
    projects: Arc<RwLock<ProjectSet>>,
    /// Features supported by the connected client.
    pub(super) client_capabilities: ClientCapabilities,
    /// Mutable editor configuration.
    pub(super) settings: ServerSettings,
    /// Diagnostic delivery selected from client capabilities.
    pub(super) diagnostics: DiagnosticDelivery,
}

impl ServerSession {
    /// Open one initialized language server session.
    pub(super) fn open(
        params: &lsp::InitializeParams,
        host: Host,
        executor: Arc<Executor>,
        trace: &Trace,
    ) -> jsonrpc::Result<Self> {
        let folders = trace.span("folders.resolve", || {
            let mut folders = Self::editor_folders(params)?;
            if folders.is_empty() {
                folders.push(Self::initial_path(params)?);
            }

            Ok::<_, jsonrpc::Error>(folders)
        })?;

        // select client behavior
        let client_capabilities = ClientCapabilities::try_from(params)?;
        let diagnostics = if client_capabilities.supports_pull_diagnostics {
            DiagnosticDelivery::pull(client_capabilities.supports_diagnostic_refresh)
        } else {
            DiagnosticDelivery::push()
        };

        let session = Self {
            executor,
            host,
            projects: Arc::new(RwLock::new(ProjectSet::default())),
            client_capabilities,
            settings: ServerSettings::default(),
            diagnostics,
        };

        // register editor folders and open their declared source roots
        for folder in folders {
            trace.span("project.open", || {
                session.open_editor_folder(&folder, trace)
            })?;
        }

        Ok(session)
    }

    /// Resolve one workspace by its exact project root.
    pub(super) fn workspace(&self, root: &Path) -> jsonrpc::Result<Arc<Workspace>> {
        let root = Self::normalize(root)?;
        let projects = self.projects.read();
        let workspace = projects.get(&root).map(Project::workspace);

        workspace.ok_or_else(|| {
            jsonrpc::Error::invalid_params(format!("TS++ project is not open: {}", root.display()))
        })
    }

    /// Select the project workspace containing one physical path.
    pub(super) fn select_workspace(&self, path: &Path) -> jsonrpc::Result<Arc<Workspace>> {
        let path = Self::normalize(path)?;
        let projects = self.projects.read();
        let workspace = projects.select(&path)?.map(Project::workspace);

        workspace.ok_or_else(|| {
            jsonrpc::Error::invalid_params(format!("no TS++ project owns {}", path.display()))
        })
    }

    /// Close every workspace and flush artifact cache writes.
    pub(super) fn shutdown(&self) -> jsonrpc::Result<()> {
        // close workspaces before flushing their artifact writes
        for workspace in self.workspaces() {
            workspace.close();
        }

        self.host.flush_artifact_cache().map_err(internal_error)
    }

    /// Resolve the project workspace and editor revision for one source path.
    pub(super) fn revision(&self, path: &Path) -> jsonrpc::Result<(Arc<Workspace>, Revision)> {
        let path = Self::normalize(path)?;
        let projects = self.projects.read();
        let project = projects.select(&path)?.ok_or_else(|| {
            jsonrpc::Error::invalid_params(format!("no TS++ project owns {}", path.display()))
        })?;
        let workspace = project.workspace();
        let revision = project.revision()?;

        Ok((workspace, revision))
    }

    /// Resolve one physical or builtin source for queries.
    pub(super) fn resolve_query_file(&self, uri: &lsp::Uri) -> jsonrpc::Result<Option<QueryFile>> {
        self.projects.read().resolve_query_file(uri)
    }

    /// Open one editor document through its nearest project.
    pub(super) fn open_document(
        &self,
        path: &Path,
        uri: lsp::Uri,
        version: i32,
        text: String,
        trace: &Trace,
    ) -> jsonrpc::Result<Arc<Workspace>> {
        let path = Self::normalize(path)?;
        if self.projects.read().select(&path)?.is_none() {
            let directory = path
                .parent()
                .ok_or_else(|| jsonrpc::Error::invalid_params("document path has no parent"))?;
            let root = SourceRoot::discover(self.host.files().as_ref(), directory)
                .map(PathBuf::from)
                .map_err(internal_error)?;
            let root = Self::canonicalize(&root)?;
            let project = Project::open(root, self.host.clone(), self.executor.clone(), trace)?;
            self.projects.write().insert(project);
        }

        let mut projects = self.projects.write();
        let project = projects.select_mut(&path)?.ok_or_else(|| {
            jsonrpc::Error::invalid_params(format!("no TS++ project owns {}", path.display()))
        })?;
        project.open_document(path, uri, version, text, trace)?;
        let workspace = project.workspace();

        Ok(workspace)
    }

    /// Apply changes to one open editor document.
    pub(super) fn change_document(
        &self,
        path: &Path,
        uri: &lsp::Uri,
        version: i32,
        changes: &[TextChange],
        trace: &Trace,
    ) -> jsonrpc::Result<Arc<Workspace>> {
        let path = Self::normalize(path)?;
        let mut projects = self.projects.write();
        let project = projects.select_mut(&path)?.ok_or_else(|| {
            jsonrpc::Error::invalid_params(format!("no TS++ project owns {}", path.display()))
        })?;
        project.change_document(&path, uri, version, changes, trace)?;

        Ok(project.workspace())
    }

    /// Save one open editor document.
    pub(super) fn save_document(
        &self,
        path: &Path,
        text: Option<String>,
        trace: &Trace,
    ) -> jsonrpc::Result<Arc<Workspace>> {
        let path = Self::normalize(path)?;
        let mut projects = self.projects.write();
        let project = projects.select_mut(&path)?.ok_or_else(|| {
            jsonrpc::Error::invalid_params(format!("no TS++ project owns {}", path.display()))
        })?;
        project.save_document(&path, text, trace)?;

        Ok(project.workspace())
    }

    /// Close one editor document and return projects released with it.
    pub(super) fn close_document(
        &self,
        path: &Path,
    ) -> jsonrpc::Result<(Arc<Workspace>, Vec<PathBuf>)> {
        let path = Self::normalize(path)?;
        let mut projects = self.projects.write();
        let project = projects.select_mut(&path)?.ok_or_else(|| {
            jsonrpc::Error::invalid_params(format!("no TS++ project owns {}", path.display()))
        })?;
        project.close_document(&path)?;
        let workspace = project.workspace();
        let removed = projects.remove_unowned();

        Ok((workspace, removed))
    }

    /// Return whether one editor document is open.
    pub(super) fn is_open(&self, path: &Path) -> jsonrpc::Result<bool> {
        let path = Self::normalize(path)?;
        let projects = self.projects.read();
        let is_open = projects
            .select(&path)?
            .is_some_and(|project| project.is_open(&path));

        Ok(is_open)
    }

    /// Return project state shared with diagnostic delivery.
    pub(super) fn projects(&self) -> Arc<RwLock<ProjectSet>> {
        self.projects.clone()
    }

    /// Read one Builtin Package file by its canonical URI.
    pub(super) fn read_builtin_file(&self, uri: &lsp::Uri) -> jsonrpc::Result<Arc<File>> {
        if uri.scheme().as_str() != TSPP_URI_SCHEME {
            return Err(jsonrpc::Error::invalid_params(format!(
                "unsupported source URI scheme: {}",
                uri.scheme()
            )));
        }

        // resolve the exact project and source revision
        let file = self
            .projects
            .read()
            .resolve_query_file(uri)?
            .map(|file| file.file)
            .ok_or_else(|| {
                jsonrpc::Error::invalid_params(format!("source URI is not tracked: {uri:?}"))
            })?;

        Ok(file)
    }

    /// Reload physical state for every opened project.
    pub(super) fn reload(&self) -> jsonrpc::Result<()> {
        self.projects.write().reload()
    }

    /// Reconcile selected physical paths and return changed project workspaces.
    pub(super) fn reconcile(&self, paths: Vec<PathBuf>) -> jsonrpc::Result<Vec<Arc<Workspace>>> {
        let paths = paths
            .into_iter()
            .map(|path| Self::normalize(&path))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        self.projects.write().reconcile(paths)
    }

    /// Register one editor folder and open its declared source root.
    pub(super) fn open_editor_folder(
        &self,
        path: &Path,
        trace: &Trace,
    ) -> jsonrpc::Result<Option<Arc<Workspace>>> {
        let path = Self::canonicalize(path)?;
        let workspace = {
            let projects = self.projects.read();

            projects.select(&path)?.map(Project::workspace)
        };
        let workspace = if let Some(workspace) = workspace {
            Some(workspace)
        } else {
            let root =
                SourceRoot::discover(self.host.files().as_ref(), &path).map_err(internal_error)?;
            match root {
                SourceRoot::Declared(root) => {
                    let root = Self::canonicalize(&root)?;
                    Some(self.open_root(root, trace)?)
                }
                SourceRoot::Implicit(_) => None,
            }
        };

        // retain client ownership after workspace resolution
        self.projects.write().add_folder(path);

        Ok(workspace)
    }

    /// Open one project from its exact source root.
    fn open_root(&self, root: PathBuf, trace: &Trace) -> jsonrpc::Result<Arc<Workspace>> {
        if let Some(project) = self.projects.read().get(&root) {
            return Ok(project.workspace());
        }

        // build the project outside the shared project lock
        let project = Project::open(root, self.host.clone(), self.executor.clone(), trace)?;

        // retain the first project opened concurrently for this root
        let workspace = self.projects.write().insert(project).workspace();

        Ok(workspace)
    }

    /// Return every opened project workspace.
    pub(super) fn workspaces(&self) -> Vec<Arc<Workspace>> {
        self.projects.read().workspaces()
    }

    /// Return the artifact workers available to this server session.
    pub(super) fn worker_count(&self) -> usize {
        self.executor.worker_count()
    }

    /// Return one opened workspace's current editor revision.
    pub(super) fn workspace_revision(&self, workspace: &Workspace) -> jsonrpc::Result<Revision> {
        let projects = self.projects.read();
        let project = projects.get(workspace.root()).ok_or_else(|| {
            jsonrpc::Error::invalid_params(format!(
                "TS++ project is not open: {}",
                workspace.root().display()
            ))
        })?;

        project.revision()
    }

    /// Clone open documents when one project remains at an exact editor revision.
    pub(super) fn documents(
        &self,
        workspace: &Workspace,
        revision: Revision,
    ) -> jsonrpc::Result<HashMap<PathBuf, lsp::VersionedTextDocumentIdentifier>> {
        let documents = self
            .projects
            .read()
            .documents(workspace.root(), revision)?
            .ok_or_else(jsonrpc::Error::content_modified)?;

        Ok(documents)
    }

    /// Close projects owned only by one removed editor folder.
    pub(super) fn close_editor_folder(&self, folder: &Path) -> jsonrpc::Result<Vec<PathBuf>> {
        let folder = Self::normalize(folder)?;
        let removed = self.projects.write().remove_folder(&folder);

        Ok(removed)
    }

    /// Read distinct editor workspace folders in client order.
    fn editor_folders(params: &lsp::InitializeParams) -> jsonrpc::Result<Vec<PathBuf>> {
        let mut folders = Vec::new();
        if let Some(workspace_folders) = params.workspace_folders.as_ref() {
            for folder in workspace_folders {
                let path = folder
                    .uri
                    .to_file_path()
                    .map(|path| path.into_owned())
                    .ok_or_else(|| {
                        jsonrpc::Error::invalid_params("workspace folder URI is not a file URI")
                    })?;
                let path = Self::canonicalize(&path)?;
                if !folders.contains(&path) {
                    folders.push(path);
                }
            }
        }

        Ok(folders)
    }

    /// Select the initial path used when no workspace folder is available.
    #[allow(deprecated)]
    fn initial_path(params: &lsp::InitializeParams) -> jsonrpc::Result<PathBuf> {
        let path = if let Some(uri) = params.root_uri.as_ref() {
            uri.to_file_path()
                .map(|path| path.into_owned())
                .ok_or_else(|| jsonrpc::Error::invalid_params("root URI is not a file URI"))?
        } else if let Some(path) = params.root_path.clone().map(PathBuf::from) {
            path
        } else {
            std::env::current_dir().map_err(|error| {
                jsonrpc::Error::invalid_params(format!(
                    "failed to resolve current directory for initialize: {error}"
                ))
            })?
        };

        Self::canonicalize(&path)
    }

    /// Canonicalize one workspace path.
    fn canonicalize(path: &Path) -> jsonrpc::Result<PathBuf> {
        fs::canonicalize(path).map_err(|error| {
            jsonrpc::Error::invalid_params(format!(
                "failed to canonicalize workspace root {}: {error}",
                path.display()
            ))
        })
    }

    /// Normalize one existing or newly removed source path.
    fn normalize(path: &Path) -> jsonrpc::Result<PathBuf> {
        match fs::canonicalize(path) {
            Ok(path) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(jsonrpc::Error::invalid_params(format!(
                    "failed to canonicalize workspace path {}: {error}",
                    path.display()
                )));
            }
        }

        // canonicalize the nearest ancestor retained by the filesystem
        for ancestor in path.ancestors().skip(1) {
            let canonical_ancestor = match fs::canonicalize(ancestor) {
                Ok(ancestor) => ancestor,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(jsonrpc::Error::invalid_params(format!(
                        "failed to canonicalize workspace path {}: {error}",
                        path.display()
                    )));
                }
            };
            let suffix = path.strip_prefix(ancestor).map_err(internal_error)?;

            return Ok(canonical_ancestor.join(suffix));
        }

        Self::canonicalize(path)
    }
}

/// LSP features supported by the connected client.
#[derive(Debug)]
pub(super) struct ClientCapabilities {
    /// Whether workspace configuration requests are supported.
    pub(super) supports_configuration: bool,
    /// Whether watched files can be registered dynamically.
    pub(super) supports_dynamic_file_watching: bool,
    /// Whether code action data payloads are supported.
    pub(super) supports_code_action_data: bool,
    /// Whether code action edits can be resolved lazily.
    pub(super) supports_code_action_edit_resolve: bool,
    /// Whether completion label details are supported.
    pub(super) supports_completion_label_details: bool,
    /// Whether completion details can be resolved lazily.
    pub(super) supports_completion_resolve: bool,
    /// Whether pull diagnostics are supported.
    pub(super) supports_pull_diagnostics: bool,
    /// Whether diagnostic refresh requests are supported.
    pub(super) supports_diagnostic_refresh: bool,
    /// Code lens commands implemented by the client bridge.
    code_lens_commands: Vec<CodeLensCommand>,
}

impl TryFrom<&lsp::InitializeParams> for ClientCapabilities {
    type Error = jsonrpc::Error;

    /// Decode relevant capabilities from initialization parameters.
    fn try_from(params: &lsp::InitializeParams) -> Result<Self, Self::Error> {
        let initialization_options = params
            .initialization_options
            .clone()
            .map(from_value::<InitializationOptions>)
            .transpose()
            .map_err(|error| {
                jsonrpc::Error::invalid_params(format!(
                    "invalid TS++ initialization options: {error}"
                ))
            })?
            .unwrap_or_default();
        let text_document = params.capabilities.text_document.as_ref();
        let code_action = text_document.and_then(|text| text.code_action.as_ref());
        let completion_item = text_document
            .and_then(|text| text.completion.as_ref())
            .and_then(|completion| completion.completion_item.as_ref());
        let workspace = params.capabilities.workspace.as_ref();

        // read protocol capabilities used by response conversion
        let supports_configuration = workspace
            .and_then(|workspace| workspace.configuration)
            .unwrap_or(false);
        let supports_dynamic_file_watching = workspace
            .and_then(|workspace| workspace.did_change_watched_files)
            .and_then(|watched_files| watched_files.dynamic_registration)
            .unwrap_or(false);
        let supports_code_action_data = code_action
            .and_then(|capabilities| capabilities.data_support)
            .unwrap_or(false);
        let supports_code_action_edit_resolve = code_action
            .and_then(|capabilities| capabilities.resolve_support.as_ref())
            .is_some_and(|resolve| resolve.properties.iter().any(|property| property == "edit"));
        let supports_completion_label_details = completion_item
            .and_then(|completion| completion.label_details_support)
            .unwrap_or(false);
        let completion_resolve_properties = ["detail", "documentation", "additionalTextEdits"];
        let supports_completion_resolve = completion_item
            .and_then(|completion| completion.resolve_support.as_ref())
            .is_some_and(|support| {
                completion_resolve_properties.iter().all(|required| {
                    support
                        .properties
                        .iter()
                        .any(|property| property == required)
                })
            });
        let supports_pull_diagnostics = text_document
            .and_then(|text| text.diagnostic.as_ref())
            .is_some();
        let supports_diagnostic_refresh = workspace
            .and_then(|workspace| workspace.diagnostic.as_ref())
            .and_then(|diagnostic| diagnostic.refresh_support)
            .unwrap_or(false);

        Ok(Self {
            supports_configuration,
            supports_dynamic_file_watching,
            supports_code_action_data,
            supports_code_action_edit_resolve,
            supports_completion_label_details,
            supports_completion_resolve,
            supports_pull_diagnostics,
            supports_diagnostic_refresh,
            code_lens_commands: initialization_options.code_lens_commands,
        })
    }
}

impl ClientCapabilities {
    /// Return whether any code lens action is executable by the client.
    pub(super) fn supports_code_lenses(&self) -> bool {
        !self.code_lens_commands.is_empty()
    }

    /// Return whether one code lens action is executable by the client.
    pub(super) fn supports_code_lens(&self, action: &query::CodeLensAction) -> bool {
        let command = match action {
            query::CodeLensAction::References { .. } => CodeLensCommand::References,
            query::CodeLensAction::Implementations { .. } => CodeLensCommand::Implementations,
        };

        self.code_lens_commands.contains(&command)
    }
}

/// Mutable language server settings.
#[derive(Debug)]
pub(super) struct ServerSettings {
    /// Whether auto import completions are enabled.
    completion_auto_imports: AtomicBool,
    /// Whether parameter name inlay hints are enabled.
    parameter_inlay_hints: AtomicBool,
    /// Whether inferred type inlay hints are enabled.
    type_inlay_hints: AtomicBool,
}

impl ServerSettings {
    /// Refresh settings from the connected client.
    pub(super) async fn refresh(&self, client: &Client) -> jsonrpc::Result<()> {
        let values = client.configuration(Self::configuration_items()).await?;

        // update completion settings
        let completion = values
            .first()
            .ok_or_else(|| internal_error("client omitted completion configuration"))?;
        if !completion.is_null() {
            let completion =
                from_value::<CompletionSettings>(completion.clone()).map_err(internal_error)?;
            if let Some(auto_imports) = completion.auto_imports {
                self.completion_auto_imports
                    .store(auto_imports, Ordering::Relaxed);
            }
        }

        // update inlay hint settings
        let inlay_hints = values
            .get(1)
            .ok_or_else(|| internal_error("client omitted inlay hint configuration"))?;
        if !inlay_hints.is_null() {
            let inlay_hints =
                from_value::<InlayHintSettings>(inlay_hints.clone()).map_err(internal_error)?;
            if let Some(parameter_hints) = inlay_hints.parameter_hints {
                self.parameter_inlay_hints
                    .store(parameter_hints, Ordering::Relaxed);
            }
            if let Some(type_hints) = inlay_hints.type_hints {
                self.type_inlay_hints.store(type_hints, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// Return whether auto import completions are enabled.
    pub(super) fn completion_auto_imports(&self) -> bool {
        self.completion_auto_imports.load(Ordering::Relaxed)
    }

    /// Return whether parameter name inlay hints are enabled.
    pub(super) fn parameter_inlay_hints(&self) -> bool {
        self.parameter_inlay_hints.load(Ordering::Relaxed)
    }

    /// Return whether inferred type inlay hints are enabled.
    pub(super) fn type_inlay_hints(&self) -> bool {
        self.type_inlay_hints.load(Ordering::Relaxed)
    }

    /// Build the configuration request for mutable settings.
    fn configuration_items() -> Vec<lsp::ConfigurationItem> {
        vec![
            lsp::ConfigurationItem {
                scope_uri: None,
                section: Some("tspp.completion".to_string()),
            },
            lsp::ConfigurationItem {
                scope_uri: None,
                section: Some("tspp.inlayHints".to_string()),
            },
        ]
    }
}

impl Default for ServerSettings {
    /// Create default language server settings.
    fn default() -> Self {
        Self {
            completion_auto_imports: AtomicBool::new(true),
            parameter_inlay_hints: AtomicBool::new(true),
            type_inlay_hints: AtomicBool::new(true),
        }
    }
}

/// Destack-specific client initialization options.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct InitializationOptions {
    /// Code lens commands implemented by the client bridge.
    code_lens_commands: Vec<CodeLensCommand>,
}

/// One code lens command implemented by a client bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CodeLensCommand {
    /// Show references at one declaration.
    References,
    /// Show implementations at one declaration.
    Implementations,
}

/// Configuration for completion behavior.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompletionSettings {
    /// Whether auto import completions are enabled.
    auto_imports: Option<bool>,
}

/// Configuration for inlay hint behavior.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InlayHintSettings {
    /// Whether parameter name hints are enabled.
    parameter_hints: Option<bool>,
    /// Whether inferred type hints are enabled.
    type_hints: Option<bool>,
}
