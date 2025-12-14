use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_workspace::Program;

/// Errors during the lint phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Lint)]
pub enum LintError {
    /// Wait for task dependency.
    #[error(code = "EL000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EL001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },
}
