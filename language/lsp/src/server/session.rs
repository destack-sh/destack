use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
use destack_artifact::MemoryBlobStore;
use destack_lsp_server::{Client, UriExt, jsonrpc};
use destack_lsp_types as lsp;
use destack_query as query;
use destack_repository::{
    DestackLayoutOverride, Environment, Execution, Settings, SourceRoot, open_repository_from_fs,
};
use destack_session::Executor;
use destack_source::{FileSystem, OverlayFileSystem, PhysicalFileSystem};
use destack_workspace::Workspace;
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::from_value;

use super::internal_error;
use crate::query::DiagnosticDelivery;

/// State installed after one language server initialization.
#[derive(Debug)]
pub(super) struct ServerSession {
    /// Artifact executor shared by every project session.
    executor: Arc<Executor>,
    /// Shared editor overlay used by every project.
    file_system: Arc<OverlayFileSystem>,
    /// Projects discovered for the current editor workspace.
    projects: RwLock<ProjectSet>,
    /// Features supported by the connected client.
    pub(super) client_capabilities: ClientCapabilities,
    /// Mutable editor configuration.
    pub(super) settings: ServerSettings,
    /// Diagnostic delivery selected from client capabilities.
    pub(super) diagnostics: DiagnosticDelivery,
}

/// Projects discovered from the current editor workspace folders.
#[derive(Debug, Default)]
struct ProjectSet {
    /// Client-provided workspace folders.
    editor_folders: Vec<PathBuf>,
    /// Discovered semantic projects ordered by source root.
    projects: Vec<Project>,
}

/// One independently configured semantic project.
#[derive(Debug)]
struct Project {
    /// The project workspace.
    workspace: Arc<Workspace>,
    /// Editor documents retaining this project.
    documents: Vec<PathBuf>,
}

impl ProjectSet {
    /// Return the most specific project containing one normalized path.
    fn select(&self, path: &Path) -> Option<&Project> {
        let index = self.select_index(path)?;

        Some(&self.projects[index])
    }

    /// Return the most specific mutable project containing one normalized path.
    fn select_mut(&mut self, path: &Path) -> Option<&mut Project> {
        let index = self.select_index(path)?;

        Some(&mut self.projects[index])
    }

    /// Return the most specific project index containing one normalized path.
    fn select_index(&self, path: &Path) -> Option<usize> {
        self.projects
            .iter()
            .enumerate()
            .filter(|(_, project)| project.contains(path))
            .max_by_key(|(_, project)| project.workspace.root().components().count())
            .map(|(index, _)| index)
    }

    /// Return the project at one exact source root.
    fn get(&self, root: &Path) -> Option<&Project> {
        self.projects
            .binary_search_by(|project| project.workspace.root().cmp(root))
            .ok()
            .map(|index| &self.projects[index])
    }

    /// Insert one project unless its source root is already open.
    fn insert(&mut self, project: Project) -> &Project {
        match self
            .projects
            .binary_search_by(|existing| existing.workspace.root().cmp(project.workspace.root()))
        {
            // merge ownership into the project already opened for this root
            Ok(index) => {
                for document in project.documents {
                    self.projects[index].open_document(document);
                }

                &self.projects[index]
            }

            // preserve source-root order for stable workspace iteration
            Err(index) => {
                self.projects.insert(index, project);

                &self.projects[index]
            }
        }
    }

    /// Retain the project containing one opened editor document.
    fn open_document(&mut self, path: PathBuf) -> Option<Arc<Workspace>> {
        let project = self.select_mut(&path)?;
        project.open_document(path);

        Some(project.workspace())
    }

    /// Release one closed editor document and remove newly unowned projects.
    fn close_document(&mut self, path: &Path) -> Vec<PathBuf> {
        if let Some(project) = self.select_mut(path) {
            project.close_document(path);
        }

        self.remove_unowned()
    }

    /// Add one client-provided workspace folder.
    fn add_folder(&mut self, folder: PathBuf) {
        if !self.editor_folders.contains(&folder) {
            self.editor_folders.push(folder);
        }
    }

    /// Remove one folder and every project no longer owned by the client.
    fn remove_folder(&mut self, folder: &Path) -> Vec<PathBuf> {
        let Some(index) = self
            .editor_folders
            .iter()
            .position(|candidate| candidate == folder)
        else {
            return Vec::new();
        };
        self.editor_folders.remove(index);

        self.remove_unowned()
    }

    /// Return every opened project workspace in source-root order.
    fn workspaces(&self) -> Vec<Arc<Workspace>> {
        self.projects
            .iter()
            .map(|project| project.workspace.clone())
            .collect()
    }

    /// Remove every project outside all editor and document ownership.
    fn remove_unowned(&mut self) -> Vec<PathBuf> {
        let editor_folders = &self.editor_folders;
        let mut removed = Vec::new();

        // remove projects outside every remaining editor folder and document
        self.projects.retain(|project| {
            let is_owned = project.is_owned(editor_folders);
            if !is_owned {
                removed.push(project.workspace.root().to_path_buf());
            }

            is_owned
        });

        removed
    }
}

impl Project {
    /// Open one project from its exact source root.
    fn open(
        root: PathBuf,
        file_system: Arc<OverlayFileSystem>,
        executor: Arc<Executor>,
    ) -> jsonrpc::Result<Self> {
        let repository = open_repository_from_fs(
            root.clone(),
            file_system.clone(),
            Environment::capture_process(),
            Settings::default(),
            DestackLayoutOverride::default(),
        )
        .map_err(internal_error)?;
        #[cfg(test)]
        let repository = repository.with_blob_store(Arc::new(MemoryBlobStore::new()));
        let workspace = Workspace::new(Arc::new(repository), Some(file_system), executor)
            .map_err(internal_error)?;

        Ok(Self {
            workspace: Arc::new(workspace),
            documents: Vec::new(),
        })
    }

    /// Return whether this project contains one normalized path.
    fn contains(&self, path: &Path) -> bool {
        self.workspace.contains(path)
    }

    /// Return whether this project and one editor folder contain one another.
    fn overlaps(&self, folder: &Path) -> bool {
        let root = self.workspace.root();

        root.starts_with(folder) || folder.starts_with(root)
    }

    /// Return whether an editor folder or document retains this project.
    fn is_owned(&self, editor_folders: &[PathBuf]) -> bool {
        !self.documents.is_empty() || editor_folders.iter().any(|folder| self.overlaps(folder))
    }

    /// Retain one opened editor document.
    fn open_document(&mut self, path: PathBuf) {
        if !self.documents.contains(&path) {
            self.documents.push(path);
        }
    }

    /// Release one closed editor document.
    fn close_document(&mut self, path: &Path) {
        if let Some(index) = self.documents.iter().position(|document| document == path) {
            self.documents.remove(index);
        }
    }

    /// Return the project workspace.
    fn workspace(&self) -> Arc<Workspace> {
        self.workspace.clone()
    }
}

impl ServerSession {
    /// Open one initialized language server session.
    pub(super) fn open(params: &lsp::InitializeParams) -> jsonrpc::Result<Self> {
        let mut folders = Self::editor_folders(params)?;
        if folders.is_empty() {
            folders.push(Self::initial_path(params)?);
        }

        let physical_file_system = Arc::new(PhysicalFileSystem::new());
        let file_system = Arc::new(OverlayFileSystem::with_inner(physical_file_system));
        let executor = Executor::new(Execution::Threaded, Executor::default_worker_count())
            .map_err(internal_error)?;
        let client_capabilities = ClientCapabilities::try_from(params)?;
        let diagnostics = if client_capabilities.supports_pull_diagnostics {
            DiagnosticDelivery::pull(client_capabilities.supports_diagnostic_refresh)
        } else {
            DiagnosticDelivery::push()
        };

        let session = Self {
            executor,
            file_system,
            projects: RwLock::new(ProjectSet::default()),
            client_capabilities,
            settings: ServerSettings::default(),
            diagnostics,
        };

        // register editor folders and open their declared source roots
        for folder in folders {
            session.open_editor_folder(&folder)?;
        }

        Ok(session)
    }

    /// Resolve the project workspace that owns one source path.
    pub(super) fn workspace(&self, path: &Path) -> jsonrpc::Result<Arc<Workspace>> {
        let path = Self::normalize(path)?;
        let workspace = self.projects.read().select(&path).map(Project::workspace);

        workspace.ok_or_else(|| {
            jsonrpc::Error::invalid_params(format!("no Destack project owns {}", path.display()))
        })
    }

    /// Open one editor document through its nearest project.
    pub(super) fn open_document(&self, path: &Path) -> jsonrpc::Result<Arc<Workspace>> {
        let path = Self::normalize(path)?;
        let workspace = self.projects.write().open_document(path.clone());
        if let Some(workspace) = workspace {
            return Ok(workspace);
        }

        let directory = path
            .parent()
            .ok_or_else(|| jsonrpc::Error::invalid_params("document path has no parent"))?;
        let root = SourceRoot::discover(self.file_system.as_ref(), directory)
            .map(PathBuf::from)
            .map_err(internal_error)?;
        let root = Self::canonicalize(&root)?;
        let mut project = Project::open(root, self.file_system.clone(), self.executor.clone())?;
        project.open_document(path);
        let workspace = self.projects.write().insert(project).workspace();

        Ok(workspace)
    }

    /// Close one editor document and return projects released with it.
    pub(super) fn close_document(&self, path: &Path) -> jsonrpc::Result<Vec<PathBuf>> {
        let path = Self::normalize(path)?;
        let removed = self.projects.write().close_document(&path);

        Ok(removed)
    }

    /// Register one editor folder and open its declared source root.
    pub(super) fn open_editor_folder(
        &self,
        path: &Path,
    ) -> jsonrpc::Result<Option<Arc<Workspace>>> {
        let path = Self::canonicalize(path)?;
        let workspace = self.projects.read().select(&path).map(Project::workspace);
        let workspace = if let Some(workspace) = workspace {
            Some(workspace)
        } else {
            let root =
                SourceRoot::discover(self.file_system.as_ref(), &path).map_err(internal_error)?;
            match root {
                SourceRoot::Declared(root) => {
                    let root = Self::canonicalize(&root)?;
                    Some(self.open_root(root)?)
                }
                SourceRoot::Implicit(_) => None,
            }
        };

        // retain client ownership after workspace resolution
        self.projects.write().add_folder(path);

        Ok(workspace)
    }

    /// Open one project from its exact source root.
    fn open_root(&self, root: PathBuf) -> jsonrpc::Result<Arc<Workspace>> {
        if let Some(project) = self.projects.read().get(&root) {
            return Ok(project.workspace());
        }

        // build the project outside the shared project lock
        let project = Project::open(root, self.file_system.clone(), self.executor.clone())?;

        // retain the first project opened concurrently for this root
        let workspace = self.projects.write().insert(project).workspace();

        Ok(workspace)
    }

    /// Return every opened project workspace.
    pub(super) fn workspaces(&self) -> Vec<Arc<Workspace>> {
        self.projects.read().workspaces()
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
        if let Ok(path) = fs::canonicalize(path) {
            return Ok(path);
        }

        let Some(parent) = path.parent() else {
            return Self::canonicalize(path);
        };
        let Some(file_name) = path.file_name() else {
            return Self::canonicalize(path);
        };
        let parent = Self::canonicalize(parent)?;

        Ok(parent.join(file_name))
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
                    "invalid Destack initialization options: {error}"
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
                section: Some("destack.completion".to_string()),
            },
            lsp::ConfigurationItem {
                scope_uri: None,
                section: Some("destack.inlayHints".to_string()),
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
