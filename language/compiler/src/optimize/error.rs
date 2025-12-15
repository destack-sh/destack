use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Errors during the optimize phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Optimize)]
pub enum OptimizeError {
    /// Wait for task dependency.
    #[error(code = "EO000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EO001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Optimization is impossible for this node.
    #[error(code = "EO002", message = "unsupported construct")]
    UnsupportedConstruct { node: GlobalNodeIdAny },

    /// Unsupported optimization.
    #[error(code = "EO003", message = "unsupported optimization")]
    UnsupportedOptimization { node: GlobalNodeIdAny },

    /// Undefined behavior possible.
    #[error(code = "EO004", message = "possible undefined behavior: {behavior}")]
    PossibleUndefinedBehavior {
        node: GlobalNodeIdAny,
        behavior: String,
    },
}
