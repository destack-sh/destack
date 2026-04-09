use crate::{
    CompileError, DiagnosticAnchor, DiagnosticDefinition, RequirementError, RequirementSet,
};
use destack_compiler_macros::DefineError;
use destack_dir::AnchoredGlobalNodeId;
use destack_source::ModuleId;
use destack_workspace::Repository;

/// Errors during the generate phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Generate)]
pub enum GenerateError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / requirement
    // -------------------------------------------------------------------------
    /// Wait for artifact requirement.
    #[error(code = "EG000", r#yield)]
    Yield { requirement: RequirementSet },

    /// Yield requirement has failed.
    #[error(code = "EG001", yield_failed)]
    UnsatisfiedRequirement { requirement: RequirementSet },

    /// Task was skipped due to stale versions.
    #[error(code = "EG002", message = "task skipped")]
    Skipped,

    // -------------------------------------------------------------------------
    // 1xx: Target / setup
    // -------------------------------------------------------------------------
    /// Unsupported target/output format.
    #[error(code = "EG100", message = "unsupported target: {target}")]
    UnsupportedTarget { module: ModuleId, target: String },

    /// Unresolved function reference.
    #[error(code = "EG101", message = "unresolved function: {name}")]
    UnresolvedFunction { module: ModuleId, name: String },

    // -------------------------------------------------------------------------
    // 2xx: Type issues
    // -------------------------------------------------------------------------
    /// Unsupported type for codegen.
    #[error(code = "EG200", message = "unsupported type")]
    UnsupportedType {
        module: ModuleId,
        node: Option<AnchoredGlobalNodeId>,
    },

    /// Missing type information.
    #[error(code = "EG201", message = "missing type")]
    MissingType {
        module: ModuleId,
        node: Option<AnchoredGlobalNodeId>,
    },

    // -------------------------------------------------------------------------
    // 3xx: Construct issues
    // -------------------------------------------------------------------------
    /// Unsupported construct (instruction, expression, etc.).
    #[error(code = "EG300", message = "unsupported construct: {message}")]
    UnsupportedConstruct {
        module: ModuleId,
        node: Option<AnchoredGlobalNodeId>,
        message: String,
    },

    /// Unexpected construct (wrong node type).
    #[error(code = "EG301", message = "unexpected construct: {message}")]
    UnexpectedConstruct {
        module: ModuleId,
        node: Option<AnchoredGlobalNodeId>,
        message: String,
    },

    /// Unresolved construct (not fully resolved before codegen).
    #[error(code = "EG302", message = "unresolved construct")]
    UnresolvedConstruct {
        module: ModuleId,
        node: Option<AnchoredGlobalNodeId>,
    },

    // -------------------------------------------------------------------------
    // 4xx: Bounds / access
    // -------------------------------------------------------------------------
    /// Out of bounds access (tuple/array element index).
    #[error(code = "EG400", message = "index {index} out of bounds (len {len})")]
    OutOfBounds {
        module: ModuleId,
        node: Option<AnchoredGlobalNodeId>,
        index: u32,
        len: usize,
    },

    // -------------------------------------------------------------------------
    // 9xx: Internal
    // -------------------------------------------------------------------------
    /// Internal codegen error.
    #[error(code = "EG900", message = "internal error: {message}")]
    Internal { module: ModuleId, message: String },
}
