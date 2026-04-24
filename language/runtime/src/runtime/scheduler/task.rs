use destack_engine as engine;
use serde::{Deserialize, Serialize};

use crate::runtime::engine::LiveContinuation;

/// Opaque task identifier used by the event loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskId(u64);

impl TaskId {
    /// Create a new task identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw task identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Scheduling status for a task or job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Ready to run.
    Ready,
    /// Waiting on an external event.
    Waiting,
    /// Completed and ready for cleanup.
    Completed,
}

/// Task metadata for event loop execution.
#[derive(Debug)]
pub struct Task {
    /// Task identifier used for ordering and logging.
    pub id: TaskId,
    /// Runnable continuation for this task.
    pub runnable: LiveContinuation,
    /// Resume payload passed back into the executor.
    pub resume_value: engine::MaterializedValue,
    /// Current scheduling status.
    pub status: TaskStatus,
    /// Priority value for event loop ordering, higher values run first.
    pub priority: u8,
}
