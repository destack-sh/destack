use crate::{CompileError, CompileTask};

/// Event emitted by the compiler.
#[derive(Debug, Clone)]
pub enum CompilerEvent {
    /// Task started.
    TaskStarted(CompileTask),
    /// Task completed.
    TaskCompleted(CompileTask),
    /// Task failed.
    TaskFailed(CompileTask, CompileError),
}
