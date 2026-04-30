use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use destack_artifact::ArtifactOutcome;
use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_source::OverlayFileSystem;
use destack_workspace::Repository;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::SessionError;
use crate::r#loop::{SessionRunId, SessionTask};

use super::{SessionEvent, SessionEventHandler, SessionOverlay};

/// Shared session state used by session workers.
pub(crate) struct SessionState {
    /// Repository backing this source root.
    repository: Arc<Repository>,
    /// Compiler for this root.
    compiler: Arc<Compiler>,
    /// Linter for this root.
    linter: Arc<Linter>,
    /// File text projected over filesystem-backed source.
    overlay: SessionOverlay,
    /// Serialize semantic access per root.
    mutation_lock: RwLock<()>,
    /// Optional outer session event handler.
    event_handler: Option<SessionEventHandler>,
    /// Monotonic ids for session runs.
    next_run_id: AtomicU32,
}

impl std::fmt::Debug for SessionState {
    /// Format the visible session state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SessionState")
            .field("repository", &self.repository)
            .field("compiler", &self.compiler)
            .field("linter", &self.linter)
            .field("overlay", &self.overlay)
            .field("event_handler", &self.event_handler.is_some())
            .finish_non_exhaustive()
    }
}

impl SessionState {
    /// Create shared state for one session.
    pub(crate) fn new(
        repository: Arc<Repository>,
        compiler: Arc<Compiler>,
        linter: Arc<Linter>,
        overlay_file_system: Option<Arc<OverlayFileSystem>>,
        event_handler: Option<SessionEventHandler>,
    ) -> Self {
        Self {
            repository,
            compiler,
            linter,
            overlay: SessionOverlay::new(overlay_file_system),
            mutation_lock: RwLock::new(()),
            event_handler,
            next_run_id: AtomicU32::new(1),
        }
    }

    /// Enter a coherent read section for this session.
    pub(crate) fn enter_query(&self) -> RwLockReadGuard<'_, ()> {
        self.mutation_lock.read()
    }

    /// Enter a mutation section for this session.
    pub(crate) fn enter_mutation(&self) -> RwLockWriteGuard<'_, ()> {
        self.mutation_lock.write()
    }

    /// Return the repository for this session.
    pub(crate) fn repository(&self) -> Arc<Repository> {
        self.repository.clone()
    }

    /// Return the compiler for this session.
    pub(crate) fn compiler(&self) -> Arc<Compiler> {
        self.compiler.clone()
    }

    /// Return the linter for this session.
    pub(crate) fn linter(&self) -> Arc<Linter> {
        self.linter.clone()
    }

    /// Return the overlay for this session.
    pub(crate) fn overlay(&self) -> &SessionOverlay {
        &self.overlay
    }

    /// Emit one outer session event when a handler is installed.
    pub(crate) fn emit_event(&self, event: SessionEvent) {
        if let Some(handler) = &self.event_handler {
            handler(event);
        }
    }

    /// Allocate the next session run id.
    pub(crate) fn next_run_id(&self) -> SessionRunId {
        let run_id = self.next_run_id.fetch_add(1, Ordering::Relaxed);

        SessionRunId(run_id)
    }

    /// Return the terminal artifact outcome for one task when it already exists.
    pub(crate) fn artifact_outcome(
        &self,
        task: SessionTask,
    ) -> Result<Option<ArtifactOutcome>, SessionError> {
        // no revision binding means the artifact has not been provided
        let Some(version) = self
            .repository()
            .artifact_version(task.revision, &task.key)?
        else {
            return Ok(None);
        };

        // revision bindings must point at a terminal store entry
        let Some(outcome) = self.repository().artifact_store().outcome(&version) else {
            return Err(SessionError::Internal {
                detail: format!("artifact version is missing from store: {version:?}"),
            });
        };

        Ok(Some(outcome))
    }
}
