use std::sync::Arc;
use std::time::Duration;

use crate::{StatsSnapshot, Task, TaskError, TaskId, TaskPhase};

/// Event emitted by the compiler during compilation.
#[derive(Debug, Clone)]
pub enum CompilerEvent {
    /// Task started processing.
    TaskStarted {
        task_id: TaskId,
        task: Task,
        phase: TaskPhase,
        /// Human-readable description of the task (e.g., "src/main.ds").
        description: String,
    },
    /// Task completed successfully.
    TaskCompleted {
        task_id: TaskId,
        task: Task,
        phase: TaskPhase,
        elapsed: Duration,
        /// Human-readable description of the task.
        description: String,
    },
    /// Task failed with an error.
    TaskFailed {
        task_id: TaskId,
        task: Task,
        phase: TaskPhase,
        error: TaskError,
    },
    /// Task yielded waiting for a dependency.
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
        /// Human-readable description of the task.
        description: String,
    },
    /// Compilation started.
    CompilationStarted { worker_count: u16 },
    /// Compilation finished.
    CompilationFinished {
        /// Compilation statistics snapshot.
        stats: StatsSnapshot,
    },
}

/// Type alias for the event handler callback.
pub type CompilerEventHandler = Arc<dyn Fn(CompilerEvent) + Send + Sync>;
