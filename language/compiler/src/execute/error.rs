use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_dir::GlobalNodeIdAny;
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
    #[error(code = "EX002", message = "unsupported construct for comptime execution")]
    UnsupportedConstruct { node: GlobalNodeIdAny },
}
