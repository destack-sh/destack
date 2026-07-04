use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use destack_artifact::ArtifactOutcome;
use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_query::Indexer;
use destack_repository::{Repository, Trace};
use parking_lot::Mutex;

use crate::SessionError;
use crate::executor::{ArtifactRunId, Task};

use super::{SessionEvent, SessionEventHandler};

/// Shared session state used by session workers.
pub(crate) struct SessionState {
    /// Repository backing this source root.
    repository: Arc<Repository>,
    /// Compiler for this root.
    compiler: Arc<Compiler>,
    /// Linter for this root.
    linter: Arc<Linter>,
    /// Indexer for this root.
    indexer: Arc<Indexer>,
    /// Optional outer session event handler.
    event_handler: Option<SessionEventHandler>,
    /// Monotonic ids for session runs.
    next_run_id: AtomicU32,
    /// The trace of the latest finished run.
    last_trace: Mutex<Option<Arc<Trace>>>,
}

impl std::fmt::Debug for SessionState {
    /// Format the visible session state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SessionState")
            .field("repository", &self.repository)
            .field("compiler", &self.compiler)
            .field("linter", &self.linter)
            .field("indexer", &self.indexer)
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
        indexer: Arc<Indexer>,
        event_handler: Option<SessionEventHandler>,
    ) -> Self {
        Self {
            repository,
            compiler,
            linter,
            indexer,
            event_handler,
            next_run_id: AtomicU32::new(1),
            last_trace: Mutex::new(None),
        }
    }

    /// Record the trace of one finished run.
    pub(crate) fn set_last_trace(&self, trace: Arc<Trace>) {
        *self.last_trace.lock() = Some(trace);
    }

    /// Return the trace of the latest finished run.
    pub(crate) fn last_trace(&self) -> Option<Arc<Trace>> {
        self.last_trace.lock().clone()
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

    /// Return the indexer for this session.
    pub(crate) fn indexer(&self) -> Arc<Indexer> {
        self.indexer.clone()
    }

    /// Emit one outer session event when a handler is installed.
    pub(crate) fn emit_event(&self, event: SessionEvent) {
        if let Some(handler) = &self.event_handler {
            handler(event);
        }
    }

    /// Allocate the next session run id.
    pub(crate) fn next_run_id(&self) -> ArtifactRunId {
        let run_id = self.next_run_id.fetch_add(1, Ordering::Relaxed);

        ArtifactRunId(run_id)
    }

    /// Return the terminal artifact outcome for one task when it already exists.
    pub(crate) fn artifact_outcome(
        &self,
        task: Task,
    ) -> Result<Option<ArtifactOutcome>, SessionError> {
        let repository = self.repository();

        // no exact binding means the task has not completed in this revision
        let Some(version) = repository.artifact_binding(task.revision, &task.key)? else {
            return Ok(None);
        };

        // revision bindings must point at a terminal store entry
        let Some(outcome) = repository.artifact_table().outcome(&version) else {
            return Err(SessionError::Internal {
                detail: format!("artifact version is missing from store: {version:?}"),
            });
        };

        Ok(Some(outcome))
    }
}
