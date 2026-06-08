use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_query as query;
use destack_repository::{DestackLayoutOverride, Environment, Repository, Revision, Settings};
use destack_session::{SessionEventHandler, open_repository_from_fs};
use destack_source::{FileSystem, Uri};
use destack_workspace::workspace::FileView;
use destack_workspace::{self as workspace};
use parking_lot::Mutex;

use crate::DaemonError;

use super::RootLeaseTable;

/// Workspace state owned by one daemon process.
#[derive(Debug)]
pub struct Workspace {
    /// Repository loaded for this workspace.
    pub repository: Arc<Repository>,
    /// Live workspace facade.
    pub(super) workspace: Arc<workspace::Workspace>,
    /// Root leases held by protocol clients.
    root_lease_table: RootLeaseTable,
}

impl Workspace {
    /// Create daemon workspace state for a repository.
    pub fn new(
        repository: Arc<Repository>,
        roots: Vec<PathBuf>,
        worker_limit: usize,
        session_event_handler: Option<SessionEventHandler>,
    ) -> Result<Self, DaemonError> {
        let workspace = workspace::Workspace::new(
            repository.clone(),
            None,
            roots,
            worker_limit,
            session_event_handler,
        )?;

        Ok(Self {
            repository,
            workspace: Arc::new(workspace),
            root_lease_table: RootLeaseTable::default(),
        })
    }

    /// Return the workspace root.
    pub fn root(&self) -> &Path {
        self.repository.path()
    }

    /// Acquire a root lease.
    pub fn acquire_root(&self, root: &Path) -> Result<(), DaemonError> {
        self.workspace.open_root(root.to_path_buf())?;

        self.root_lease_table.acquire(root);

        Ok(())
    }

    /// Release a root lease and close when the last lease is dropped.
    pub fn release_root(&self, root: &Path) -> Result<bool, DaemonError> {
        let should_close = self.root_lease_table.release(root);

        if should_close {
            self.workspace.close_root(root)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Open one editor file.
    pub fn open_file(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        change: workspace::FileChange,
    ) -> Result<workspace::UpdateBatch, workspace::Error> {
        self.workspace.open_file(path, uri, version, change)
    }

    /// Change one editor file.
    pub fn change_file(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        change: workspace::FileChange,
    ) -> Result<workspace::UpdateBatch, workspace::Error> {
        self.workspace.change_file(path, uri, version, change)
    }

    /// Close one editor file.
    pub fn close_file(&self, path: &Path) -> Result<workspace::UpdateBatch, workspace::Error> {
        self.workspace.close_file(path)
    }

    /// Run one query for a root.
    pub fn query_root(
        &self,
        root: &Path,
        request: query::QueryRequest,
        revision: workspace::RevisionPolicy,
    ) -> Result<workspace::QueryResult, workspace::Error> {
        self.workspace.query_root(root, request, revision)
    }

    /// Return diagnostics for one root.
    pub fn root_diagnostics(
        &self,
        root: &Path,
    ) -> Result<Vec<workspace::DiagnosticView>, workspace::Error> {
        self.workspace.root_diagnostics(root)
    }

    /// Return diagnostics for one file.
    pub fn file_diagnostics(
        &self,
        path: &Path,
    ) -> Result<Option<workspace::DiagnosticView>, workspace::Error> {
        self.workspace.file_diagnostics(path)
    }

    /// Clear all workspace caches.
    pub fn clear_cache_all(&self) -> Result<(), workspace::Error> {
        self.workspace.clear_cache_all()
    }

    /// Return the current revision for one root.
    pub fn revision(&self, root: &Path) -> Result<Revision, workspace::Error> {
        self.workspace.revision(root)
    }

    /// Return one current file view.
    pub fn file_view(&self, path: &Path) -> Result<FileView, workspace::Error> {
        self.workspace.file_view(path)
    }

    /// Return the number of tracked roots.
    #[cfg(test)]
    pub fn root_count(&self) -> usize {
        self.workspace.root_count()
    }
}

/// Workspaces opened inside one daemon.
pub struct WorkspaceTable {
    /// File system used to load new workspaces.
    file_system: Arc<dyn FileSystem>,
    /// Settings used to load new workspaces.
    settings: Settings,
    /// Layout override used to load new workspaces.
    layout_override: DestackLayoutOverride,
    /// Workspaces keyed by workspace root.
    workspaces: Mutex<HashMap<PathBuf, Arc<Workspace>>>,
    /// Number of workers for each opened workspace.
    worker_limit: usize,
    /// Optional session event handler for opened workspaces.
    session_event_handler: Option<SessionEventHandler>,
}

impl std::fmt::Debug for WorkspaceTable {
    /// Format the visible workspace table state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceTable")
            .field("file_system", &"<file_system>")
            .field("settings", &self.settings)
            .field("layout_override", &self.layout_override)
            .field("workspaces", &self.workspaces)
            .field("worker_limit", &self.worker_limit)
            .field(
                "session_event_handler",
                &self.session_event_handler.is_some(),
            )
            .finish()
    }
}

impl WorkspaceTable {
    /// Create a workspace table from an initial repository.
    pub fn new(
        repository: Arc<Repository>,
        worker_limit: usize,
        session_event_handler: Option<SessionEventHandler>,
    ) -> Result<Self, DaemonError> {
        let file_system = repository.file_system().clone();
        let settings = repository.settings().clone();
        let layout_override = DestackLayoutOverride {
            home: Some(repository.layout().home.clone()),
            packages: Some(repository.layout().packages.clone()),
            workspace_cache: None,
        };
        let workspace_root = repository.path().to_path_buf();
        let roots = vec![workspace_root.clone()];
        let workspace = Arc::new(Workspace::new(
            repository.clone(),
            roots,
            worker_limit,
            session_event_handler.clone(),
        )?);
        let mut workspaces = HashMap::new();
        workspaces.insert(workspace_root, workspace);

        Ok(Self {
            file_system,
            settings,
            layout_override,
            workspaces: Mutex::new(workspaces),
            worker_limit,
            session_event_handler,
        })
    }

    /// Return an opened workspace or load it from the filesystem.
    pub fn open(&self, workspace_root: &Path) -> Result<Arc<Workspace>, DaemonError> {
        // reuse existing workspace state
        if let Some(workspace) = self.workspaces.lock().get(workspace_root).cloned() {
            return Ok(workspace);
        }

        // load the requested workspace from the shared file system
        let repository = self.load_repository(workspace_root)?;
        let workspace_root = repository.path().to_path_buf();
        let roots = vec![workspace_root.clone()];
        let workspace = Arc::new(Workspace::new(
            Arc::new(repository),
            roots,
            self.worker_limit,
            self.session_event_handler.clone(),
        )?);

        // publish the workspace unless another client raced us
        let mut workspaces = self.workspaces.lock();
        if let Some(existing) = workspaces.get(&workspace_root).cloned() {
            return Ok(existing);
        }
        workspaces.insert(workspace_root, workspace.clone());

        Ok(workspace)
    }

    /// Return one opened workspace.
    pub fn get(&self, workspace_root: &Path) -> Option<Arc<Workspace>> {
        self.workspaces.lock().get(workspace_root).cloned()
    }

    /// Return opened workspace roots.
    pub fn roots(&self) -> Vec<PathBuf> {
        self.workspaces.lock().keys().cloned().collect()
    }

    /// Return the number of opened workspaces.
    pub(crate) fn len(&self) -> usize {
        self.workspaces.lock().len()
    }

    /// Load a repository using the daemon's shared machine settings.
    fn load_repository(&self, workspace_root: &Path) -> Result<Repository, DaemonError> {
        let mut environment = Environment::capture_process();
        environment.cwd = Some(workspace_root.to_path_buf());
        let repository = open_repository_from_fs(
            workspace_root.to_path_buf(),
            self.file_system.clone(),
            environment,
            self.settings.clone(),
            self.layout_override.clone(),
        )
        .map_err(|error| DaemonError::WorkspaceOpen {
            root: workspace_root.to_path_buf(),
            detail: error.to_string(),
        })?;

        Ok(repository)
    }
}
