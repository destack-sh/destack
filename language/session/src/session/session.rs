use std::sync::Arc;
use std::thread;

use destack_repository::Repository;

use crate::SessionError;
use crate::executor::Executor;

use super::SessionState;

/// Artifact computation session for one repository.
pub struct Session {
    /// Shared session state for providers and workers.
    pub(crate) state: Arc<SessionState>,
    /// Artifact executor for this live session.
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
    /// Return the default session worker count.
    pub fn default_worker_count() -> usize {
        thread::available_parallelism().map_or(1, usize::from)
    }

    /// Create one artifact computation session.
    pub fn new(repository: Arc<Repository>, worker_count: usize) -> Result<Self, SessionError> {
        let state = Arc::new(SessionState::new(repository));

        Ok(Self {
            state: state.clone(),
            executor: Executor::new(state, worker_count)?,
        })
    }

    /// Return the repository for this session.
    pub fn repository(&self) -> Arc<Repository> {
        self.state.repository()
    }
}
