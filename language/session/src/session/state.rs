use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use destack_artifact::ArtifactOutcome;
use destack_compiler::Compiler;
use destack_linter::Linter;
use destack_query::Indexer;
use destack_repository::Repository;

use crate::executor::{ArtifactRunId, Task};
use crate::{SessionError, diagnostic};

/// Shared session state used by session workers.
pub(crate) struct SessionState {
    /// Repository read and updated by providers.
    repository: Arc<Repository>,
    /// Compiler used by this session.
    compiler: Compiler,
    /// Linter used by this session.
    linter: Linter,
    /// Indexer used by this session.
    indexer: Indexer,
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
            .field("indexer", &self.indexer)
            .finish_non_exhaustive()
    }
}

impl SessionState {
    /// Create shared state for one session.
    pub(crate) fn new(repository: Arc<Repository>) -> Self {
        let diagnostics = diagnostic::registry();

        // initialize repository consumers with the shared diagnostic registry
        let compiler = Compiler::new(repository.clone(), Arc::new(diagnostics));
        let linter = Linter::new(repository.clone());
        let indexer = Indexer::new(repository.clone());

        Self {
            repository,
            compiler,
            linter,
            indexer,
            next_run_id: AtomicU32::new(1),
        }
    }

    /// Return the repository for this session.
    pub(crate) fn repository(&self) -> Arc<Repository> {
        self.repository.clone()
    }

    /// Return the compiler for this session.
    pub(crate) fn compiler(&self) -> &Compiler {
        &self.compiler
    }

    /// Return the linter for this session.
    pub(crate) fn linter(&self) -> &Linter {
        &self.linter
    }

    /// Return the indexer for this session.
    pub(crate) fn indexer(&self) -> &Indexer {
        &self.indexer
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
        let outcome = repository.current_artifact_outcome(task.revision, &task.key)?;

        Ok(outcome)
    }
}
