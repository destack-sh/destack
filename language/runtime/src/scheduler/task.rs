use destack_vm as vm;
use serde::{Deserialize, Serialize};

use super::runnable::Runnable;

/// Opaque task identifier used by the scheduler.
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

/// Scheduling state for a task or job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Ready to run.
    Ready,
    /// Waiting on an external event.
    Waiting,
    /// Completed and ready for cleanup.
    Completed,
}

/// Scheduler task metadata for event loop execution.
#[derive(Debug)]
pub struct Task {
    /// Task identifier used for ordering and logging.
    pub id: TaskId,
    /// Runnable continuation for this task.
    pub runnable: Runnable,
    /// Resume payload passed back into the executor.
    pub resume_value: vm::Value,
    /// Current scheduling state.
    pub state: TaskState,
    // NOTE #Incomplete: priority is not used by the scheduler yet
    /// Priority value for scheduler ordering.
    pub priority: u8,
}
