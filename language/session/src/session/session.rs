use std::sync::Arc;

use tspp_repository::Repository;

use crate::SessionError;
use crate::executor::Executor;

use super::SessionState;

/// Artifact computation session for one repository.
pub struct Session {
    /// Repository-specific provider state.
    pub(crate) state: Arc<SessionState>,
    /// Shared artifact executor.
    pub(crate) executor: Arc<Executor>,
}

impl std::fmt::Debug for Session {
    /// Format the visible session state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Session")
            .field("state", &self.state)
            .field("executor", &self.executor)
            .finish_non_exhaustive()
    }
}

impl Session {
    /// Create one repository-specific artifact computation session.
    pub fn new(repository: Arc<Repository>, executor: Arc<Executor>) -> Result<Self, SessionError> {
        let repository_execution = repository.host().execution();
        let executor_execution = executor.execution();
        if repository_execution != executor_execution {
            return Err(SessionError::ExecutionMismatch {
                repository: repository_execution,
                executor: executor_execution,
            });
        }

        let id = executor.next_session_id();
        let state = Arc::new(SessionState::new(id, repository));

        Ok(Self { state, executor })
    }

    /// Return the repository for this session.
    pub fn repository(&self) -> Arc<Repository> {
        self.state.repository()
    }

    /// Return the shared artifact executor attached to this session.
    pub fn executor(&self) -> Arc<Executor> {
        self.executor.clone()
    }
}
