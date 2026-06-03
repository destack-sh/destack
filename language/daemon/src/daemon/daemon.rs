use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_session::SessionEventHandler;
use destack_source::{FileWatcher, PhysicalFileWatcher};
use destack_workspace::{Ref, Repository};

use crate::DaemonError;

use super::{DaemonWorkspace, WorkspaceTable};

/// Persistent process state for daemon clients.
#[derive(Clone)]
pub struct Daemon {
    /// Number of workers for each opened session.
    pub worker_limit: usize,
    /// Watcher implementation used by daemon-owned watches.
    pub file_watcher: Arc<dyn FileWatcher>,
    /// Opened workspaces keyed by workspace root.
    workspaces: Arc<WorkspaceTable>,
    /// Next id for private command session refs.
    next_command_session_id: Arc<AtomicU64>,
}

impl std::fmt::Debug for Daemon {
    /// Format the visible daemon state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Daemon")
            .field("worker_limit", &self.worker_limit)
            .field("file_watcher", &"<file_watcher>")
            .field("workspaces", &self.workspaces)
            .field("next_command_session_id", &self.next_command_session_id)
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
        let workspaces = WorkspaceTable::new(repository, worker_limit, session_event_handler)?;

        Ok(Self {
            worker_limit,
            file_watcher,
            workspaces: Arc::new(workspaces),
            next_command_session_id: Arc::new(AtomicU64::new(1)),
        })
    }

    /// Allocate one private session ref for a command.
    pub(crate) fn next_command_session_ref(&self, root: &Path) -> Ref {
        let id = self.next_command_session_id.fetch_add(1, Ordering::Relaxed);

        Ref::new(format!("command:{}:{id}", root.display()))
    }

    /// Open or return one workspace.
    pub(crate) fn open_workspace(
        &self,
        workspace_root: &Path,
    ) -> Result<Arc<DaemonWorkspace>, DaemonError> {
        self.workspaces.open(workspace_root)
    }

    /// Return one opened workspace.
    pub(crate) fn workspace(&self, workspace_root: &Path) -> Option<Arc<DaemonWorkspace>> {
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

    /// Return the number of tracked roots.
    #[cfg(test)]
    pub(crate) fn root_count(&self) -> usize {
        self.workspaces
            .roots()
            .iter()
            .filter_map(|root| self.workspaces.get(root))
            .map(|workspace| workspace.root_count())
            .sum()
    }
}
