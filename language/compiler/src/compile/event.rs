use crate::{CompileError, CompilerTask};

/// Event emitted by the compiler.
#[derive(Debug, Clone)]
pub enum CompilerEvent {
	/// Task started.
    TaskStarted(CompilerTask),
    /// Task completed.
    TaskCompleted(CompilerTask),
	/// Task failed.
    TaskFailed(CompilerTask, CompileError),
}
