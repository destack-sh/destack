use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_repository::{
    DestackLayoutOverride, Environment, Repository, Settings, open_repository_from_fs,
};
use destack_session::SessionEventHandler;
use destack_source::{FileSystem, FileWatcher};
use destack_workspace::LocalWorkspace;
use parking_lot::Mutex;

use crate::DaemonError;

/// One workspace opened inside a daemon process.
#[derive(Debug)]
pub struct OpenedWorkspace {
    /// Live workspace facade.
    pub(super) workspace: Arc<LocalWorkspace>,
}

impl OpenedWorkspace {
    /// Open one daemon workspace for a repository.
    pub fn new(
        repository: Arc<Repository>,
        roots: Vec<PathBuf>,
        worker_limit: usize,
        session_event_handler: Option<SessionEventHandler>,
        file_watcher: Arc<dyn FileWatcher>,
    ) -> Result<Self, DaemonError> {
        let workspace = LocalWorkspace::new(
            repository,
            None,
            Some(file_watcher),
            roots,
            worker_limit,
            session_event_handler,
        )?;

        Ok(Self {
            workspace: Arc::new(workspace),
        })
    }

    /// Return the local workspace.
    pub fn workspace(&self) -> Arc<LocalWorkspace> {
        self.workspace.clone()
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
    workspaces: Mutex<HashMap<PathBuf, Arc<OpenedWorkspace>>>,
    /// Number of workers for each opened workspace.
    worker_limit: usize,
    /// Optional session event handler for opened workspaces.
    session_event_handler: Option<SessionEventHandler>,
    /// File watcher used by opened local workspaces.
    file_watcher: Arc<dyn FileWatcher>,
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
            .field("file_watcher", &"<file_watcher>")
            .finish()
    }
}

impl WorkspaceTable {
    /// Create a workspace table from an initial repository.
    pub fn new(
        repository: Arc<Repository>,
        worker_limit: usize,
        session_event_handler: Option<SessionEventHandler>,
        file_watcher: Arc<dyn FileWatcher>,
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
        let workspace = Arc::new(OpenedWorkspace::new(
            repository.clone(),
            roots,
            worker_limit,
            session_event_handler.clone(),
            file_watcher.clone(),
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
            file_watcher,
        })
    }

    /// Return an opened workspace or load it from the filesystem.
    pub fn open(&self, workspace_root: &Path) -> Result<Arc<OpenedWorkspace>, DaemonError> {
        // reuse an already opened workspace
        if let Some(workspace) = self.workspaces.lock().get(workspace_root).cloned() {
            return Ok(workspace);
        }

        // load the requested workspace from the shared file system
        let repository = self.load_repository(workspace_root)?;
        let workspace_root = repository.path().to_path_buf();
        let roots = vec![workspace_root.clone()];
        let workspace = Arc::new(OpenedWorkspace::new(
            Arc::new(repository),
            roots,
            self.worker_limit,
            self.session_event_handler.clone(),
            self.file_watcher.clone(),
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
    pub fn get(&self, workspace_root: &Path) -> Option<Arc<OpenedWorkspace>> {
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
