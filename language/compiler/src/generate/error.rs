use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Errors during the generate phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Generate)]
pub enum GenerateError {
    /// Wait for task dependency.
    #[error(code = "EG000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EG001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Unsupported target/output format.
    #[error(code = "EG002", message = "unsupported target: {target}")]
    UnsupportedTarget {
        node: GlobalNodeIdAny,
        target: String,
    },

    /// Unsupported construct (instruction, expression, etc.).
    #[error(code = "EG003", message = "unsupported construct")]
    UnsupportedConstruct { node: GlobalNodeIdAny },

    /// Unsupported type for codegen.
    #[error(code = "EG004", message = "unsupported type")]
    UnsupportedType { node: GlobalNodeIdAny },

    /// Unexpected construct (wrong node type).
    #[error(code = "EG005", message = "unexpected construct")]
    UnexpectedConstruct { node: GlobalNodeIdAny },

    /// Unresolved construct (not fully resolved before codegen).
    #[error(code = "EG006", message = "unresolved construct")]
    UnresolvedConstruct { node: GlobalNodeIdAny },

    /// Unresolved function reference.
    #[error(code = "EG007", message = "unresolved function: {name}")]
    UnresolvedFunction { node: GlobalNodeIdAny, name: String },

    /// Missing type information.
    #[error(code = "EG008", message = "missing type")]
    MissingType { node: GlobalNodeIdAny },

    /// Out of bounds access (tuple/array element index).
    #[error(code = "EG009", message = "index {index} out of bounds (len {len})")]
    OutOfBounds {
        node: GlobalNodeIdAny,
        index: u32,
        len: usize,
    },

    /// Internal codegen error.
    #[error(code = "EG010", message = "internal error: {message}")]
    Internal {
        node: GlobalNodeIdAny,
        message: String,
    },
}
