use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_query::Query;
use destack_session::{Session, SessionEventHandler};
use destack_source::OverlayFileSystem;
use destack_workspace::Repository;

use super::LanguageServiceError;
use super::open::OpenFile;

/// Local language service used by tooling integrations.
pub struct LanguageService {
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
    pub(super) open_file_by_path: DashMap<PathBuf, OpenFile>,
    /// Overlay filesystem shared by live sessions.
    pub(super) overlay_file_system: Option<Arc<OverlayFileSystem>>,
    /// Number of workers for each opened session.
    pub(super) worker_limit: usize,
    /// Optional session event handler for local progress reporting.
    pub(super) event_handler: Option<SessionEventHandler>,
}

impl std::fmt::Debug for LanguageService {
    /// Format the visible language service state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LanguageService")
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

impl LanguageService {
    /// Create a local language service for the provided roots.
    pub fn new(
        repository: Arc<Repository>,
        overlay_file_system: Option<Arc<OverlayFileSystem>>,
        roots: Vec<PathBuf>,
        worker_limit: usize,
        event_handler: Option<SessionEventHandler>,
    ) -> Result<Self, LanguageServiceError> {
        let compiler = Arc::new(Compiler::new(repository.clone()));
        let linter = Arc::new(Linter::new(repository.clone()));
        let query = Arc::new(Query::new(repository.clone()));

        let service = Self {
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
            service.open_root(root)?;
        }

        Ok(service)
    }
}
