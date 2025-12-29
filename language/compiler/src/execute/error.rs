use crate::{
    DiagnosticAnchor, DiagnosticDefinition, LowerError, TaskDependency, TaskDependencyError,
    TaskError,
};
use destack_compiler_macros::DefineError;
use destack_dir::GlobalNodeIdAny;
use destack_source::ModuleId;
use destack_workspace::Program;

/// Errors during the execute phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Execute)]
pub enum ExecuteError {
    /// Wait for task dependency.
    #[error(code = "EX000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EX001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Unsupported construct for comptime execution.
    #[error(
        code = "EX002",
        message = "unsupported construct for comptime execution"
    )]
    UnsupportedConstruct { node: GlobalNodeIdAny },

    /// Comptime lowering failed.
    #[error(code = "EX003", message = "comptime lowering failed: {message}")]
    FailedLower {
        /// Anchor the error to a module.
        module: ModuleId,
        /// Report the lowering error.
        error: Box<LowerError>,
        /// Describe the lowering failure.
        message: String,
    },
    /// Comptime execution failed.
    #[error(code = "EX004", message = "comptime execution failed: {message}")]
    FailedExecution {
        /// Anchor the error to a module.
        module: ModuleId,
        /// Describe the execution failure.
        message: String,
    },
}
