use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Errors during the verify phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Verify)]
pub enum VerifyError {
    /// Wait for task dependency.
    #[error(code = "EV000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EV001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Unsupported node.
    #[error(code = "EV002", message = "unsupported construct")]
    UnsupportedConstruct { node: GlobalNodeIdAny },
}
