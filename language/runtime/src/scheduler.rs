use destack_vm::{Continuation, Value};

/// Scheduler task state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Ready to run.
    Ready,
    /// Waiting on an external event.
    Waiting,
    /// Completed and ready for cleanup.
    Completed,
}

/// Runtime task metadata for scheduling decisions.
#[derive(Debug)]
pub struct Task {
    /// Continuation to resume for this task.
    pub continuation: Continuation,
    /// Resume payload passed back into the VM.
    pub resume_value: Value,
    /// Current task scheduling state.
    pub state: TaskState,
    /// Priority value for scheduler ordering.
    pub priority: u8,
}

/// Scheduler for task queues and event loops.
#[derive(Debug, Default)]
pub struct Scheduler;
