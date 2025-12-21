use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_dir::{GlobalNodeIdAny, GlobalTypeId};
use destack_workspace::Program;

/// Errors during the lower phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Lower)]
pub enum LowerError {
    /// Wait for task dependency.
    #[error(code = "EM000", r#yield)]
    Yield {
        /// Carry the dependency that must be satisfied before lowering can proceed.
        dependency: TaskDependency,
    },

    /// Yield dependency has failed.
    #[error(code = "EM001", yield_failed)]
    UnsatisfiedDependency {
        /// Carry the dependency that failed to resolve.
        dependency: TaskDependency,
    },

    /// Unsupported node.
    #[error(code = "EM002", message = "unsupported construct: {message}")]
    UnsupportedConstruct {
        /// Report the offending node that cannot be lowered.
        node: GlobalNodeIdAny,
        /// Describe why the construct is unsupported.
        message: String,
    },

    /// Unsupported type.
    #[error(code = "EM003", message = "unsupported type {ty}: {message}")]
    UnsupportedType {
        /// Report the node that introduced the unsupported type.
        node: GlobalNodeIdAny,
        /// Identify the unsupported type id.
        ty: GlobalTypeId,
        /// Describe why the type is unsupported.
        message: String,
    },

    /// Missing type.
    #[error(code = "EM004", message = "missing type")]
    MissingType {
        /// Report the node that lacks type information.
        node: GlobalNodeIdAny,
    },
}
