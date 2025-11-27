use crate::{Task, TaskError};

/// Event emitted by the compiler.
#[derive(Debug, Clone)]
pub enum CompilerEvent {
    /// Task started.
    TaskStarted(Task),
    /// Task completed.
    TaskCompleted(Task),
    /// Task failed.
    TaskFailed(Task, TaskError),
}
