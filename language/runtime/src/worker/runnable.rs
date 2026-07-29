use serde::{Deserialize, Serialize};

use crate::worker::scheduler::RunnableId;

/// Currently running task or microtask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunnableScope {
    /// No task or microtask is active.
    Empty,
    /// One task is active.
    Task {
        /// Active task identifier.
        task_id: RunnableId,
    },
    /// One microtask is active.
    Microtask {
        /// Active microtask identifier.
        microtask_id: RunnableId,
    },
}

/// Runtime work that made visible scheduler progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RunnableProgress {
    /// One task progressed.
    Task {
        /// Task that progressed.
        task_id: RunnableId,
    },
    /// One microtask progressed.
    Microtask {
        /// Microtask that progressed.
        microtask_id: RunnableId,
    },
}

impl RunnableScope {
    /// Create an empty runnable scope.
    pub const fn empty() -> Self {
        Self::Empty
    }

    /// Create a task runnable scope.
    pub const fn task(task_id: RunnableId) -> Self {
        Self::Task { task_id }
    }

    /// Create a microtask runnable scope.
    pub const fn microtask(microtask_id: RunnableId) -> Self {
        Self::Microtask { microtask_id }
    }

    /// Return the current task identifier.
    pub const fn task_id(self) -> Option<RunnableId> {
        match self {
            Self::Task { task_id } => Some(task_id),
            Self::Empty | Self::Microtask { .. } => None,
        }
    }

    /// Return the current microtask identifier.
    pub const fn microtask_id(self) -> Option<RunnableId> {
        match self {
            Self::Microtask { microtask_id } => Some(microtask_id),
            Self::Empty | Self::Task { .. } => None,
        }
    }

    /// Return the progress represented by one active runnable scope.
    pub(crate) const fn progress(self) -> Option<RunnableProgress> {
        match self {
            Self::Task { task_id } => Some(RunnableProgress::task(task_id)),
            Self::Microtask { microtask_id } => Some(RunnableProgress::microtask(microtask_id)),
            Self::Empty => None,
        }
    }
}

impl RunnableProgress {
    /// Create one task progress value.
    pub(crate) const fn task(task_id: RunnableId) -> Self {
        Self::Task { task_id }
    }

    /// Create one microtask progress value.
    pub(crate) const fn microtask(microtask_id: RunnableId) -> Self {
        Self::Microtask { microtask_id }
    }
}
