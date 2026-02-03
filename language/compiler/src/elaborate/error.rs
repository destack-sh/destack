use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
    TaskSkipReason,
};
use destack_compiler_macros::DefineError;
use destack_dir::AnchoredGlobalNodeId;
use destack_workspace::Program;

/// Errors during the elaborate phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Elaborate)]
pub enum ElaborateError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / dependency
    // -------------------------------------------------------------------------
    /// Wait for task dependency.
    #[error(code = "EE000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EE001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Task was skipped due to stale versions.
    #[error(code = "EE002", message = "task skipped")]
    Skipped { reason: TaskSkipReason },

    // -------------------------------------------------------------------------
    // 1xx: Configuration
    // -------------------------------------------------------------------------
    /// Implicit collection conversions are disabled by configuration.
    #[error(
        code = "EE100",
        message = "implicit collection conversions are disabled"
    )]
    ImplicitCollectionConversion { node: AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported node.
    #[error(code = "EE900", message = "unsupported construct")]
    UnsupportedConstruct { node: AnchoredGlobalNodeId },
}
