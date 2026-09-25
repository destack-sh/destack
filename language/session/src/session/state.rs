use std::sync::Arc;

use tspp_artifact::ArtifactOutcome;
use tspp_compiler::Compiler;
use tspp_index::Indexer;
use tspp_linter::Linter;
use tspp_repository::Repository;

use crate::executor::{SessionId, Task};
use crate::{SessionError, diagnostic};

/// Repository-specific state used by artifact workers.
pub(crate) struct SessionState {
    /// Identity of this session inside its artifact executor.
    id: SessionId,
    /// Repository read and updated by providers.
    repository: Arc<Repository>,
    /// Compiler used by this session.
    compiler: Compiler,
    /// Linter used by this session.
    linter: Linter,
    /// Indexer used by this session.
    indexer: Indexer,
}

impl std::fmt::Debug for SessionState {
    /// Format the visible session state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SessionState")
            .field("id", &self.id)
            .field("repository", &self.repository)
            .field("compiler", &self.compiler)
            .field("linter", &self.linter)
            .field("indexer", &self.indexer)
            .finish_non_exhaustive()
    }
}

impl SessionState {
    /// Create shared state for one session.
    pub(crate) fn new(id: SessionId, repository: Arc<Repository>) -> Self {
        let diagnostics = diagnostic::registry();

        // initialize repository consumers with the shared diagnostic registry
        let compiler = Compiler::new(repository.clone(), Arc::new(diagnostics));
        let linter = Linter::new(repository.clone());
        let indexer = Indexer::new(repository.clone());

        Self {
            id,
            repository,
            compiler,
            linter,
            indexer,
        }
    }

    /// Return this session's executor identity.
    pub(crate) fn id(&self) -> SessionId {
        self.id
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

    /// Return the terminal artifact outcome for one task when it already exists.
    pub(crate) fn artifact_outcome(
        &self,
        task: Task,
    ) -> Result<Option<ArtifactOutcome>, SessionError> {
        if task.session != self.id {
            return Err(SessionError::Internal {
                detail: "artifact task belongs to another session".to_string(),
            });
        }

        let outcome = self
            .repository
            .current_artifact_outcome(task.revision, &task.key)?;

        Ok(outcome)
    }
}
