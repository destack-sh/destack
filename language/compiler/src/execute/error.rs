use crate::{
    DiagnosticAnchor, DiagnosticDefinition, LowerError, TaskDependency, TaskDependencyError,
    TaskError,
};
use destack_compiler_macros::DefineError;
use destack_dir::AnchoredGlobalNodeId;
use destack_source::ModuleId;
use destack_workspace::Program;

/// Errors during the execute phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Execute)]
pub enum ExecuteError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / dependency
    // -------------------------------------------------------------------------
    /// Wait for task dependency.
    #[error(code = "EX000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EX001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    // -------------------------------------------------------------------------
    // 1xx: Comptime execution errors
    // -------------------------------------------------------------------------
    /// Comptime lowering failed.
    #[error(code = "EX100", message = "comptime lowering failed: {message}")]
    FailedLower {
        /// Anchor the error to a module.
        module: ModuleId,
        /// Report the lowering error.
        error: Box<LowerError>,
        /// Describe the lowering failure.
        message: String,
    },

    /// Comptime execution failed.
    #[error(code = "EX101", message = "comptime execution failed: {message}")]
    FailedExecution {
        /// Anchor the error to a module.
        module: ModuleId,
        /// Describe the execution failure.
        message: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported construct for comptime execution.
    #[error(
        code = "EX900",
        message = "unsupported construct for comptime execution"
    )]
    UnsupportedConstruct { node: AnchoredGlobalNodeId },
}
