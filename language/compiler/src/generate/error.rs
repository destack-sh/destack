use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_dir::GlobalNodeIdAny;
use destack_source::ModuleId;
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
    UnsupportedTarget { module: ModuleId, target: String },

    /// Unresolved function reference.
    #[error(code = "EG003", message = "unresolved function: {name}")]
    UnresolvedFunction { module: ModuleId, name: String },

    /// Internal codegen error.
    #[error(code = "EG004", message = "internal error: {message}")]
    Internal { module: ModuleId, message: String },

    /// Unsupported construct (instruction, expression, etc.).
    #[error(code = "EG005", message = "unsupported construct")]
    UnsupportedConstruct {
        module: ModuleId,
        node: Option<GlobalNodeIdAny>,
    },

    /// Unsupported type for codegen.
    #[error(code = "EG006", message = "unsupported type")]
    UnsupportedType {
        module: ModuleId,
        node: Option<GlobalNodeIdAny>,
    },

    /// Unexpected construct (wrong node type).
    #[error(code = "EG007", message = "unexpected construct")]
    UnexpectedConstruct {
        module: ModuleId,
        node: Option<GlobalNodeIdAny>,
    },

    /// Unresolved construct (not fully resolved before codegen).
    #[error(code = "EG008", message = "unresolved construct")]
    UnresolvedConstruct {
        module: ModuleId,
        node: Option<GlobalNodeIdAny>,
    },

    /// Missing type information.
    #[error(code = "EG009", message = "missing type")]
    MissingType {
        module: ModuleId,
        node: Option<GlobalNodeIdAny>,
    },

    /// Out of bounds access (tuple/array element index).
    #[error(code = "EG010", message = "index {index} out of bounds (len {len})")]
    OutOfBounds {
        module: ModuleId,
        node: Option<GlobalNodeIdAny>,
        index: u32,
        len: usize,
    },
}
