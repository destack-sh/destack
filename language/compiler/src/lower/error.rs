use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Errors during the lower phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Lower)]
pub enum LowerError {
    /// Wait for task dependency.
    #[error(code = "EL000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EL001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Unsupported node.
    #[error(code = "EL002", message = "unsupported construct")]
    UnsupportedConstruct { node: GlobalNodeIdAny },
}
