use crate::{
    CompileError, DiagnosticAnchor, DiagnosticDefinition, LowerError, RequirementError,
    RequirementSet,
};
use destack_compiler_macros::DefineError;
use destack_dir::AnchoredGlobalNodeId;
use destack_source::ModuleId;
use destack_workspace::Repository;

/// Errors during the execute phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Execute)]
pub enum ExecuteError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / requirement
    // -------------------------------------------------------------------------
    /// Wait for artifact requirement.
    #[error(code = "EX000", r#yield)]
    Yield { requirement: RequirementSet },

    /// Yield requirement has failed.
    #[error(code = "EX001", yield_failed)]
    UnsatisfiedRequirement { requirement: RequirementSet },

    /// Task was skipped due to stale versions.
    #[error(code = "EX002", message = "task skipped")]
    Skipped,

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
