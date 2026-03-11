use std::sync::Arc;
use std::time::Duration;

use crate::{StatsSnapshot, Task, TaskError, TaskId, TaskPhase, TaskSkipReason};

/// Event emitted by the compiler during compilation.
#[derive(Debug, Clone)]
pub enum CompilerEvent {
    /// Task started processing.
    TaskStarted {
        task_id: TaskId,
        task: Task,
        phase: TaskPhase,
        description: String,
    },
    /// Task completed successfully.
    TaskCompleted {
        task_id: TaskId,
        task: Task,
        phase: TaskPhase,
        elapsed: Duration,
        description: String,
    },
    /// Task failed with an error.
    TaskFailed {
        task_id: TaskId,
        task: Task,
        phase: TaskPhase,
        error: TaskError,
    },
    /// Task was skipped.
    TaskSkipped {
        task_id: TaskId,
        task: Task,
        phase: TaskPhase,
        reason: TaskSkipReason,
        description: String,
    },
    /// Task yielded waiting for one build requirement.
    TaskYielded {
        task_id: TaskId,
        task: Task,
        phase: TaskPhase,
    },
    /// Task was slow (exceeded threshold).
    TaskSlow {
        task_id: TaskId,
        task: Task,
        phase: TaskPhase,
        elapsed: Duration,
        description: String,
    },
    /// Compilation started.
    CompilationStarted { worker_count: u16 },
    /// Compilation finished.
    CompilationFinished { stats: StatsSnapshot },
}

/// Type alias for the event handler callback.
pub type CompilerEventHandler = Arc<dyn Fn(CompilerEvent) + Send + Sync>;
