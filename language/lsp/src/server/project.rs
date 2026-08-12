use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::BuildId;
use destack_lsp_server::jsonrpc;
use destack_lsp_types as lsp;
use destack_repository::{
    Commit, DestackLayoutOverride, Environment, Host, Repository, Revision, Settings,
};
use destack_session::Executor;
use destack_source::{Edit, FileId, FileSystem, TextChange, apply_text_changes};
use destack_workspace::{FileSelection, Workspace};

use super::{internal_error, workspace_error};

/// Private branch used for Language Server Protocol document state.
const LSP_BRANCH: &str = "lsp";

/// One independently configured semantic project.
#[derive(Debug)]
pub(super) struct Project {
    /// The project workspace.
    workspace: Arc<Workspace>,
    /// Documents currently owned by the editor.
    documents: HashMap<PathBuf, lsp::VersionedTextDocumentIdentifier>,
}

impl Project {
    /// Open one project from its exact source root.
    pub(super) fn open(
        root: PathBuf,
        file_system: Arc<dyn FileSystem>,
        executor: Arc<Executor>,
    ) -> jsonrpc::Result<Self> {
        let build_id = BuildId::current().map_err(internal_error)?;
        let host = Host::new(build_id, Environment::capture_process(), file_system);
        let (repository, physical) = Repository::open(
            root,
            host,
            Settings::default(),
            DestackLayoutOverride::default(),
        )
        .map_err(internal_error)?;
        let workspace =
            Workspace::new(Arc::new(repository), physical, executor).map_err(internal_error)?;

        // create the private branch used by every editor document
        let revision = workspace.revision().map_err(workspace_error)?;
        workspace
            .create_branch(LSP_BRANCH.to_string(), revision)
            .map_err(workspace_error)?;

        Ok(Self {
            workspace: Arc::new(workspace),
            documents: HashMap::new(),
        })
    }

    /// Return whether this project contains one normalized path.
    pub(super) fn contains(&self, path: &Path) -> bool {
        self.workspace.contains(path)
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
    ) -> jsonrpc::Result<()> {
        if self.is_open(&path) {
            return Err(jsonrpc::Error::invalid_params(format!(
                "document is already open: {}",
                path.display()
            )));
        }

        // commit exact editor text before retaining its client identity
        self.edit(path.clone(), text)?;
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
        let text = self.text(path)?;
        let text = apply_text_changes(text, changes)
            .map_err(|error| jsonrpc::Error::invalid_params(error.to_string()))?;
        self.edit(path.to_path_buf(), text)?;

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
    ) -> jsonrpc::Result<()> {
        self.versioned_document(path)?;

        // accept clients that include complete text with the save notification
        if let Some(text) = text {
            self.edit(path.to_path_buf(), text)?;
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
    fn edit(&self, path: PathBuf, text: String) -> jsonrpc::Result<()> {
        let revision = self.revision()?;
        self.workspace
            .edit_branch(LSP_BRANCH, revision, vec![Edit::SetText { path, text }])
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
    /// Return the most specific project containing one normalized path.
    pub(super) fn select(&self, path: &Path) -> Option<&Project> {
        let index = self.select_index(path)?;

        Some(&self.projects[index])
    }

    /// Return the most specific mutable project containing one normalized path.
    pub(super) fn select_mut(&mut self, path: &Path) -> Option<&mut Project> {
        let index = self.select_index(path)?;

        Some(&mut self.projects[index])
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

    /// Reload every project and merge closed physical files into editor branches.
    pub(super) fn reload(&mut self) -> jsonrpc::Result<()> {
        for project in &mut self.projects {
            project.reload()?;
        }

        Ok(())
    }

    /// Reconcile changed physical paths through their most specific projects.
    pub(super) fn reconcile(
        &mut self,
        paths: Vec<PathBuf>,
    ) -> jsonrpc::Result<Vec<Arc<Workspace>>> {
        // group paths by their most specific project
        let mut selected = vec![Vec::new(); self.projects.len()];
        for path in paths {
            if let Some(index) = self.select_index(&path) {
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

    /// Return the most specific project index containing one normalized path.
    fn select_index(&self, path: &Path) -> Option<usize> {
        self.projects
            .iter()
            .enumerate()
            .filter(|(_, project)| project.contains(path))
            .max_by_key(|(_, project)| project.workspace.root().components().count())
            .map(|(index, _)| index)
    }
}
