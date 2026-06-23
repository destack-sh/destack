use std::path::Path;
use std::sync::Arc;

use destack_repository::Repository;
use destack_session::SessionEventHandler;
use destack_source::{FileWatcher, PhysicalFileWatcher};
use destack_workspace::protocol::{ProtocolError, ProtocolErrorCode};
use destack_workspace::{Workspace, WorkspaceRegistry};

use crate::DaemonError;

use super::{OpenedWorkspace, WorkspaceTable};

/// Persistent state for workspace server clients.
#[derive(Clone)]
pub struct Daemon {
    /// Number of workers for each opened session.
    pub worker_limit: usize,
    /// Watcher implementation used by server-owned watches.
    pub file_watcher: Arc<dyn FileWatcher>,
    /// Opened workspaces keyed by workspace root.
    workspaces: Arc<WorkspaceTable>,
}

impl std::fmt::Debug for Daemon {
    /// Format the visible daemon state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Daemon")
            .field("worker_limit", &self.worker_limit)
            .field("file_watcher", &"<file_watcher>")
            .field("workspaces", &self.workspaces)
            .finish()
    }
}

impl Daemon {
    /// Create a daemon.
    pub fn new(
        repository: Arc<Repository>,
        worker_limit: usize,
        session_event_handler: Option<SessionEventHandler>,
    ) -> Result<Self, DaemonError> {
        let file_watcher = Arc::new(PhysicalFileWatcher::new());

        Self::new_with_watcher(
            repository,
            worker_limit,
            session_event_handler,
            file_watcher,
        )
    }

    /// Create a daemon with an explicit watcher implementation.
    pub fn new_with_watcher(
        repository: Arc<Repository>,
        worker_limit: usize,
        session_event_handler: Option<SessionEventHandler>,
        file_watcher: Arc<dyn FileWatcher>,
    ) -> Result<Self, DaemonError> {
        let workspaces = WorkspaceTable::new(
            repository,
            worker_limit,
            session_event_handler,
            file_watcher.clone(),
        )?;

        Ok(Self {
            worker_limit,
            file_watcher,
            workspaces: Arc::new(workspaces),
        })
    }

    /// Open or return one workspace.
    pub(crate) fn open(&self, workspace_root: &Path) -> Result<Arc<OpenedWorkspace>, DaemonError> {
        self.workspaces.open(workspace_root)
    }

    /// Return one opened workspace.
    pub(crate) fn workspace(&self, workspace_root: &Path) -> Option<Arc<OpenedWorkspace>> {
        self.workspaces.get(workspace_root)
    }

    /// Return all opened workspace roots.
    pub fn workspace_roots(&self) -> Vec<std::path::PathBuf> {
        self.workspaces.roots()
    }

    /// Return the number of opened workspaces.
    pub fn workspace_count(&self) -> usize {
        self.workspaces.len()
    }
}

impl WorkspaceRegistry for Daemon {
    fn open(&self, workspace: &Path) -> Result<Arc<dyn Workspace>, ProtocolError> {
        let workspace = Daemon::open(self, workspace)
            .map_err(|error| ProtocolError::internal(error.to_string()))?;

        Ok(workspace.workspace())
    }

    fn workspace(&self, workspace: &Path) -> Option<Arc<dyn Workspace>> {
        self.workspace(workspace)
            .map(|workspace| workspace.workspace() as Arc<dyn Workspace>)
    }

    fn acquire(&self, workspace: &Path, root: &Path) -> Result<(), ProtocolError> {
        let workspace = Daemon::open(self, workspace)
            .map_err(|error| ProtocolError::internal(error.to_string()))?;
        workspace
            .acquire_root(root)
            .map_err(|error| ProtocolError::internal(error.to_string()))
    }

    fn release(&self, workspace: &Path, root: &Path) -> Result<(), ProtocolError> {
        let Some(workspace) = self.workspace(workspace) else {
            return Err(ProtocolError::new(
                ProtocolErrorCode::NotFound,
                "workspace is closed",
            ));
        };
        workspace
            .release_root(root)
            .map_err(|error| ProtocolError::internal(error.to_string()))?;

        Ok(())
    }
}
