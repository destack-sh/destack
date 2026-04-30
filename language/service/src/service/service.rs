use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use destack_compiler::{Compiler, CompilerOptions};
use destack_linter::Linter;
use destack_session::{Session, SessionEventHandler};
use destack_source::OverlayFileSystem;
use destack_workspace::Repository;

use super::LanguageServiceError;

/// Local language service used by tooling integrations.
pub struct LanguageService {
    /// Repository for workspace resolution.
    pub(super) repository: Arc<Repository>,
    /// Compiler used by opened sessions.
    pub(super) compiler: Arc<Compiler>,
    /// Linter used by opened sessions.
    pub(super) linter: Arc<Linter>,

    /// Optional session event handler for local progress reporting.
    pub(super) events: Option<SessionEventHandler>,
    /// Sessions keyed by root path.
    pub(super) roots: DashMap<PathBuf, Arc<Session>>,
    /// Overlay filesystem shared by live sessions.
    pub(super) overlay_file_system: Option<Arc<OverlayFileSystem>>,
}

impl std::fmt::Debug for LanguageService {
    /// Format the visible language service state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LanguageService")
            .field("repository", &self.repository)
            .field("compiler", &self.compiler)
            .field("linter", &self.linter)
            .field("session_event_handler", &self.events.is_some())
            .field("sessions_by_root", &self.roots)
            .field("overlay_file_system", &self.overlay_file_system.is_some())
            .finish()
    }
}

impl LanguageService {
    /// Create a local language service for the provided roots.
    pub fn new(
        repository: Arc<Repository>,
        roots: Vec<PathBuf>,
    ) -> Result<Self, LanguageServiceError> {
        Self::with_options(repository, None, roots, CompilerOptions::default(), None)
    }
}
