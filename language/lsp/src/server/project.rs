use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tspp_core::BlobId;
use tspp_lsp_server::{UriExt, jsonrpc};
use tspp_lsp_types as lsp;
use tspp_repository::{Commit, Host, Repository, Revision, Settings, StorageLayoutOverride, Trace};
use tspp_session::Executor;
use tspp_source::{Edit, FileId, TextChange, Uri, apply_text_changes};
use tspp_workspace::{FileSelection, QueryFile, Workspace};

use super::{internal_error, workspace_error};

/// Private branch used for Language Server Protocol document state.
const LSP_BRANCH: &str = "lsp";
/// Canonical Builtin Package URI prefix.
const TSPP_URI_PREFIX: &str = "tspp://";
/// URI scheme served by the TS++ virtual document provider.
pub(crate) const TSPP_URI_SCHEME: &str = "tspp";

/// Stable identity of one canonical LSP project root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectId(BlobId);

impl ProjectId {
    /// Derive a project identity from one canonical root path.
    pub(crate) fn from_root(root: &Path) -> Self {
        Self(BlobId::for_bytes(root.as_os_str().as_encoded_bytes()))
    }

    /// Qualify one canonical source URI for this project.
    pub(crate) fn qualify(self, uri: &Uri) -> jsonrpc::Result<lsp::Uri> {
        let source = uri
            .as_ref()
            .strip_prefix(TSPP_URI_PREFIX)
            .ok_or_else(|| internal_error(format!("source URI is not a builtin URI: {uri}")))?;
        let qualified = format!("{TSPP_URI_SCHEME}://{}/{source}", self.0);

        qualified.parse().map_err(internal_error)
    }
}

/// One independently configured semantic project.
#[derive(Debug)]
pub(super) struct Project {
    /// Stable identity derived from the canonical project root.
    id: ProjectId,
    /// The project workspace.
    workspace: Arc<Workspace>,
    /// Documents currently owned by the editor.
    documents: HashMap<PathBuf, lsp::VersionedTextDocumentIdentifier>,
}

impl Project {
    /// Open one project from its exact source root.
    pub(super) fn open(
        root: PathBuf,
        host: Host,
        executor: Arc<Executor>,
        trace: &Trace,
    ) -> jsonrpc::Result<Self> {
        // import the physical repository
        let (repository, physical) = trace
            .span("repository.open", || {
                Repository::open(
                    root,
                    host,
                    Settings::default(),
                    StorageLayoutOverride::default(),
                )
            })
            .map_err(internal_error)?;
        let repository = Arc::new(repository);

        // restore cached artifacts valid at this revision
        trace
            .span("artifact_cache.restore", || {
                repository.restore_artifacts(physical, executor.worker_count())
            })
            .map_err(internal_error)?;

        // construct live workspace state
        let workspace = trace
            .span("workspace.create", || {
                Workspace::new(repository, physical, executor)
            })
            .map_err(internal_error)?;

        // create the private branch used by every editor document
        let revision = workspace.revision().map_err(workspace_error)?;
        workspace
            .create_branch(LSP_BRANCH.to_string(), revision)
            .map_err(workspace_error)?;

        let id = ProjectId::from_root(workspace.root());

        Ok(Self {
            id,
            workspace: Arc::new(workspace),
            documents: HashMap::new(),
        })
    }

    /// Return whether this project's package index contains one physical path.
    fn contains(&self, path: &Path) -> jsonrpc::Result<bool> {
        let revision = self.revision()?;
        let package = self
            .workspace
            .nearest_package(revision, path)
            .map_err(workspace_error)?;

        Ok(package.is_some())
    }

    /// Return whether this project and one editor folder contain one another.
    pub(super) fn overlaps(&self, folder: &Path) -> bool {
        let root = self.workspace.root();

        root.starts_with(folder) || folder.starts_with(root)
    }

    /// Return whether an editor folder or document retains this project.
    pub(super) fn is_owned(&self, editor_folders: &[PathBuf]) -> bool {
        !self.documents.is_empty() || editor_folders.iter().any(|folder| self.overlaps(folder))
    }

    /// Return the project workspace.
    pub(super) fn workspace(&self) -> Arc<Workspace> {
        self.workspace.clone()
    }

    /// Return the current editor revision.
    pub(super) fn revision(&self) -> jsonrpc::Result<Revision> {
        self.workspace
            .branch_revision(LSP_BRANCH)
            .map_err(workspace_error)
    }

    /// Return whether one editor document is open.
    pub(super) fn is_open(&self, path: &Path) -> bool {
        self.documents.contains_key(path)
    }

    /// Clone open document identities at the current editor revision.
    fn documents(&self) -> HashMap<PathBuf, lsp::VersionedTextDocumentIdentifier> {
        self.documents.clone()
    }

    /// Open one editor document and publish its exact content.
    pub(super) fn open_document(
        &mut self,
        path: PathBuf,
        uri: lsp::Uri,
        version: i32,
        text: String,
        trace: &Trace,
    ) -> jsonrpc::Result<()> {
        if self.is_open(&path) {
            return Err(jsonrpc::Error::invalid_params(format!(
                "document is already open: {}",
                path.display()
            )));
        }

        // commit exact editor text before retaining its client identity
        trace.span("document.publish", || self.edit(path.clone(), text, trace))?;
        let document = lsp::VersionedTextDocumentIdentifier { uri, version };
        let replaced = self.documents.insert(path.clone(), document);
        if replaced.is_some() {
            return Err(internal_error(format!(
                "document map replaced an unopened path: {}",
                path.display()
            )));
        }

        Ok(())
    }

    /// Apply ordered incremental changes to one editor document.
    pub(super) fn change_document(
        &mut self,
        path: &Path,
        uri: &lsp::Uri,
        version: i32,
        changes: &[TextChange],
        trace: &Trace,
    ) -> jsonrpc::Result<()> {
        let document = self.versioned_document(path)?;

        // require stable client identity and a monotonic document version
        if &document.uri != uri {
            return Err(jsonrpc::Error::invalid_params(format!(
                "document URI changed while open: {}",
                path.display()
            )));
        }
        if version <= document.version {
            return Err(jsonrpc::Error::content_modified());
        }

        // apply ordered changes against the exact editor branch text
        let text = trace.span("document.read", || self.text(path))?;
        let text = trace
            .span("document.apply", || apply_text_changes(text, changes))
            .map_err(|error| jsonrpc::Error::invalid_params(error.to_string()))?;
        trace.span("document.publish", || {
            self.edit(path.to_path_buf(), text, trace)
        })?;

        // advance the client version only after committing semantic state
        let document = self.documents.get_mut(path).ok_or_else(|| {
            internal_error(format!(
                "document map lost an open path: {}",
                path.display()
            ))
        })?;
        document.version = version;

        Ok(())
    }

    /// Save one editor document to physical workspace state.
    pub(super) fn save_document(
        &mut self,
        path: &Path,
        text: Option<String>,
        trace: &Trace,
    ) -> jsonrpc::Result<()> {
        self.versioned_document(path)?;

        // accept clients that include complete text with the save notification
        if let Some(text) = text {
            trace.span("document.publish", || {
                self.edit(path.to_path_buf(), text, trace)
            })?;
        }

        // persist only this document from the private editor branch
        let revision = self.revision()?;
        let physical = self.workspace.revision().map_err(workspace_error)?;
        self.workspace
            .save_branch(
                LSP_BRANCH,
                revision,
                physical,
                FileSelection::Paths(vec![path.to_path_buf()]),
            )
            .map_err(workspace_error)?;

        Ok(())
    }

    /// Close one editor document and restore its physical source state.
    pub(super) fn close_document(&mut self, path: &Path) -> jsonrpc::Result<()> {
        self.versioned_document(path)?;

        // replace this document's private state with current physical state
        let revision = self.revision()?;
        let physical = self.workspace.revision().map_err(workspace_error)?;
        self.workspace
            .restore_branch(
                LSP_BRANCH,
                revision,
                physical,
                FileSelection::Paths(vec![path.to_path_buf()]),
            )
            .map_err(workspace_error)?;

        // release the client identity after its branch state is restored
        self.documents.remove(path).ok_or_else(|| {
            internal_error(format!(
                "document map lost an open path: {}",
                path.display()
            ))
        })?;

        Ok(())
    }

    /// Reload physical state and merge closed files into the editor branch.
    pub(super) fn reload(&mut self) -> jsonrpc::Result<()> {
        let revision = self.revision()?;
        let Some(commit) = self.workspace.reload().map_err(workspace_error)? else {
            return Ok(());
        };

        self.merge(revision, commit)?;

        Ok(())
    }

    /// Reconcile selected physical paths into the editor branch.
    pub(super) fn reconcile(&mut self, paths: Vec<PathBuf>) -> jsonrpc::Result<bool> {
        let revision = self.revision()?;
        let Some(commit) = self.workspace.reconcile(paths).map_err(workspace_error)? else {
            return Ok(false);
        };

        self.merge(revision, commit)
    }

    /// Merge closed physical files from one commit into the editor branch.
    fn merge(&mut self, revision: Revision, commit: Commit) -> jsonrpc::Result<bool> {
        // merge physical changes for files not owned by open editor documents
        let paths = commit
            .changes
            .iter()
            .filter(|change| {
                let path = self.workspace.root().join(&change.path);

                !self.is_open(&path)
            })
            .map(|change| PathBuf::from(&change.path))
            .collect::<Vec<_>>();
        if paths.is_empty() {
            return Ok(false);
        }

        self.workspace
            .restore_branch(
                LSP_BRANCH,
                revision,
                commit.after,
                FileSelection::Paths(paths),
            )
            .map_err(workspace_error)?;

        Ok(true)
    }

    /// Return one exact open document identity.
    fn versioned_document(
        &self,
        path: &Path,
    ) -> jsonrpc::Result<&lsp::VersionedTextDocumentIdentifier> {
        self.documents.get(path).ok_or_else(|| {
            jsonrpc::Error::invalid_params(format!("document is not open: {}", path.display()))
        })
    }

    /// Replace one editor file at the current private revision.
    fn edit(&self, path: PathBuf, text: String, trace: &Trace) -> jsonrpc::Result<()> {
        let revision = self.revision()?;
        self.workspace
            .edit_branch(
                LSP_BRANCH,
                revision,
                vec![Edit::SetText { path, text }],
                trace,
            )
            .map_err(workspace_error)?;

        Ok(())
    }

    /// Read one editor file's current text.
    fn text(&self, path: &Path) -> jsonrpc::Result<String> {
        let revision = self.revision()?;
        let logical_path = path.strip_prefix(self.workspace.root()).map_err(|_| {
            jsonrpc::Error::invalid_params(format!(
                "document is outside project root: {}",
                path.display()
            ))
        })?;
        let file_id = FileId::from_logical_path(logical_path);
        let mut files = self
            .workspace
            .read_files(revision, vec![file_id])
            .map_err(workspace_error)?;
        let file = files.pop().ok_or_else(|| {
            jsonrpc::Error::invalid_params(format!("document is missing: {}", path.display()))
        })?;

        Ok(file.text().to_string())
    }
}

/// Projects discovered from the current editor workspace folders.
#[derive(Debug, Default)]
pub(crate) struct ProjectSet {
    /// Client-provided workspace folders.
    editor_folders: Vec<PathBuf>,
    /// Discovered semantic projects ordered by source root.
    projects: Vec<Project>,
}

impl ProjectSet {
    /// Return the project owning one normalized path.
    pub(super) fn select(&self, path: &Path) -> jsonrpc::Result<Option<&Project>> {
        let Some(index) = self.select_index(path)? else {
            return Ok(None);
        };

        Ok(Some(&self.projects[index]))
    }

    /// Return the mutable project owning one normalized path.
    pub(super) fn select_mut(&mut self, path: &Path) -> jsonrpc::Result<Option<&mut Project>> {
        let Some(index) = self.select_index(path)? else {
            return Ok(None);
        };

        Ok(Some(&mut self.projects[index]))
    }

    /// Return the project at one exact source root.
    pub(super) fn get(&self, root: &Path) -> Option<&Project> {
        self.projects
            .binary_search_by(|project| project.workspace.root().cmp(root))
            .ok()
            .map(|index| &self.projects[index])
    }

    /// Insert one project unless its source root is already open.
    pub(super) fn insert(&mut self, project: Project) -> &Project {
        match self
            .projects
            .binary_search_by(|existing| existing.workspace.root().cmp(project.workspace.root()))
        {
            // existing project
            Ok(index) => &self.projects[index],

            // new project
            Err(index) => {
                self.projects.insert(index, project);

                &self.projects[index]
            }
        }
    }

    /// Add one client-provided workspace folder.
    pub(super) fn add_folder(&mut self, folder: PathBuf) {
        if !self.editor_folders.contains(&folder) {
            self.editor_folders.push(folder);
        }
    }

    /// Remove one folder and every project no longer owned by the client.
    pub(super) fn remove_folder(&mut self, folder: &Path) -> Vec<PathBuf> {
        let Some(index) = self
            .editor_folders
            .iter()
            .position(|candidate| candidate == folder)
        else {
            return Vec::new();
        };

        // remove client ownership before collecting unowned projects
        self.editor_folders.remove(index);

        self.remove_unowned()
    }

    /// Return every opened project workspace in source-root order.
    pub(super) fn workspaces(&self) -> Vec<Arc<Workspace>> {
        self.projects.iter().map(Project::workspace).collect()
    }

    /// Resolve one physical or builtin source for queries.
    pub(super) fn resolve_query_file(&self, uri: &lsp::Uri) -> jsonrpc::Result<Option<QueryFile>> {
        // select builtin sources through their encoded project identity
        let (project, source_uri) = if uri.scheme().as_str() == TSPP_URI_SCHEME {
            // decode the project and canonical source path
            let qualified = uri.as_str().strip_prefix(TSPP_URI_PREFIX).ok_or_else(|| {
                jsonrpc::Error::invalid_params(format!("unsupported source URI: {uri:?}"))
            })?;
            let (project_id, source) = qualified.split_once('/').ok_or_else(|| {
                jsonrpc::Error::invalid_params(format!("builtin URI has no source path: {uri:?}"))
            })?;
            let project_id = project_id.parse::<BlobId>().map(ProjectId).map_err(|_| {
                jsonrpc::Error::invalid_params(format!(
                    "builtin URI has invalid project id: {uri:?}"
                ))
            })?;

            // select the exact open project
            let project = self
                .projects
                .iter()
                .find(|project| project.id == project_id)
                .ok_or_else(|| {
                    jsonrpc::Error::invalid_params(format!(
                        "builtin URI belongs to an unopened project: {uri:?}"
                    ))
                })?;

            // restore the repository source URI
            let source_uri = Uri::from_string(format!("{TSPP_URI_SCHEME}://{source}"));

            (project, source_uri)
        }
        // select physical sources through their owning project
        else {
            let path = uri.to_file_path().ok_or_else(|| {
                jsonrpc::Error::invalid_params(format!("unsupported source URI: {uri:?}"))
            })?;
            let project = self.select(&path)?.ok_or_else(|| {
                jsonrpc::Error::invalid_params(format!("no TS++ project owns {}", path.display()))
            })?;

            (project, Uri::from_path(path))
        };
        let revision = project.revision()?;

        project
            .workspace
            .resolve_query_file(revision, source_uri)
            .map_err(workspace_error)
    }

    /// Reload every project and merge closed physical files into editor branches.
    pub(super) fn reload(&mut self) -> jsonrpc::Result<()> {
        for project in &mut self.projects {
            project.reload()?;
        }

        Ok(())
    }

    /// Reconcile changed physical paths through their owning projects.
    pub(super) fn reconcile(
        &mut self,
        paths: Vec<PathBuf>,
    ) -> jsonrpc::Result<Vec<Arc<Workspace>>> {
        // group paths by their owning project
        let mut selected = vec![Vec::new(); self.projects.len()];
        for path in paths {
            if let Some(index) = self.select_index(&path)? {
                selected[index].push(path);
            }
        }

        // reconcile each selected project and retain semantic changes
        let mut changed = Vec::new();
        for (project, paths) in self.projects.iter_mut().zip(selected) {
            if !paths.is_empty() && project.reconcile(paths)? {
                changed.push(project.workspace());
            }
        }

        Ok(changed)
    }

    /// Return one opened project's current editor revision.
    pub(crate) fn revision(&self, root: &Path) -> Option<jsonrpc::Result<Revision>> {
        self.get(root).map(Project::revision)
    }

    /// Clone document identities when one project remains at an exact revision.
    pub(crate) fn documents(
        &self,
        root: &Path,
        revision: Revision,
    ) -> jsonrpc::Result<Option<HashMap<PathBuf, lsp::VersionedTextDocumentIdentifier>>> {
        let Some(project) = self.get(root) else {
            return Ok(None);
        };
        if project.revision()? != revision {
            return Ok(None);
        }

        Ok(Some(project.documents()))
    }

    /// Remove every project outside all editor and document ownership.
    pub(super) fn remove_unowned(&mut self) -> Vec<PathBuf> {
        let editor_folders = &self.editor_folders;
        let mut removed = Vec::new();

        // collect roots while retaining every client-owned project
        self.projects.retain(|project| {
            let is_owned = project.is_owned(editor_folders);
            if !is_owned {
                removed.push(project.workspace.root().to_path_buf());
            }

            is_owned
        });

        removed
    }

    /// Return the project index owning one normalized path.
    fn select_index(&self, path: &Path) -> jsonrpc::Result<Option<usize>> {
        // retain the project that owns an open editor document
        if let Some(index) = self
            .projects
            .iter()
            .position(|project| project.is_open(path))
        {
            return Ok(Some(index));
        }

        // select the single project containing the source package
        let mut selected = None;
        for (index, project) in self.projects.iter().enumerate() {
            if !project.contains(path)? {
                continue;
            }

            // reject ambiguous project membership
            if selected.is_some() {
                return Err(internal_error(format!(
                    "source belongs to multiple TS++ projects: {}",
                    path.display()
                )));
            }

            selected = Some(index);
        }

        Ok(selected)
    }
}
