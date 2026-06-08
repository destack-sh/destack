use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_query::Query;
use destack_repository::Repository;
use destack_session::{Session, SessionEventHandler};
use destack_source::OverlayFileSystem;

use crate::diagnostic::Error;
use crate::file::OpenFile;

/// Local workspace used by tooling integrations.
pub struct Workspace {
    /// Repository for workspace resolution.
    pub(super) repository: Arc<Repository>,
    /// Compiler used by opened sessions.
    pub(super) compiler: Arc<Compiler>,
    /// Linter used by opened sessions.
    pub(super) linter: Arc<Linter>,
    /// Query provider used by opened sessions.
    pub(super) query: Arc<Query>,
    /// Sessions keyed by root path.
    pub(super) roots: DashMap<PathBuf, Arc<Session>>,
    /// Open files keyed by source path.
    pub(crate) open_file_by_path: DashMap<PathBuf, OpenFile>,
    /// Overlay filesystem shared by live sessions.
    pub(crate) overlay_file_system: Option<Arc<OverlayFileSystem>>,
    /// Number of workers for each opened session.
    pub(super) worker_limit: usize,
    /// Optional session event handler for local progress reporting.
    pub(super) event_handler: Option<SessionEventHandler>,
}

impl std::fmt::Debug for Workspace {
    /// Format the visible workspace state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Workspace")
            .field("repository", &self.repository)
            .field("compiler", &self.compiler)
            .field("linter", &self.linter)
            .field("query", &self.query)
            .field("sessions_by_root", &self.roots)
            .field("open_file_by_path", &self.open_file_by_path.len())
            .field("overlay_file_system", &self.overlay_file_system.is_some())
            .field("worker_limit", &self.worker_limit)
            .field("event_handler", &self.event_handler.is_some())
            .finish()
    }
}

impl Workspace {
    /// Create a local workspace for the provided roots.
    pub fn new(
        repository: Arc<Repository>,
        overlay_file_system: Option<Arc<OverlayFileSystem>>,
        roots: Vec<PathBuf>,
        worker_limit: usize,
        event_handler: Option<SessionEventHandler>,
    ) -> Result<Self, Error> {
        let compiler = Arc::new(Compiler::new(repository.clone()));
        let linter = Arc::new(Linter::new(repository.clone()));
        let query = Arc::new(Query::new(repository.clone()));

        let workspace = Self {
            repository,
            compiler,
            linter,
            query,
            roots: dashmap::DashMap::new(),
            open_file_by_path: dashmap::DashMap::new(),
            overlay_file_system,
            worker_limit,
            event_handler,
        };

        for root in roots {
            workspace.open_root(root)?;
        }

        Ok(workspace)
    }

    /// Return opened roots as a stable path list.
    pub(crate) fn root_paths(&self) -> Vec<PathBuf> {
        self.roots.iter().map(|entry| entry.key().clone()).collect()
    }
}
