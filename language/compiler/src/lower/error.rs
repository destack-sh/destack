use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_dir::{AnchoredGlobalNodeId, GlobalTypeId};
use destack_source::ModuleId;
use destack_workspace::Program;

/// Errors during the lower phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Lower)]
pub enum LowerError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / dependency
    // -------------------------------------------------------------------------
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

    // -------------------------------------------------------------------------
    // 1xx: Type issues
    // -------------------------------------------------------------------------
    /// Unsupported type.
    #[error(code = "EM100", message = "unsupported type {ty}: {message}")]
    UnsupportedType {
        /// Report the node that introduced the unsupported type.
        node: AnchoredGlobalNodeId,
        /// Identify the unsupported type id.
        ty: GlobalTypeId,
        /// Describe why the type is unsupported.
        message: String,
    },

    /// Missing type.
    #[error(code = "EM101", message = "missing type")]
    MissingType {
        /// Report the node that lacks type information.
        node: AnchoredGlobalNodeId,
    },

    // -------------------------------------------------------------------------
    // 2xx: Construct issues
    // -------------------------------------------------------------------------
    /// Unsupported node.
    #[error(code = "EM200", message = "unsupported construct: {message}")]
    UnsupportedConstruct {
        /// Report the offending node that cannot be lowered.
        node: AnchoredGlobalNodeId,
        /// Describe why the construct is unsupported.
        message: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Internal
    // -------------------------------------------------------------------------
    /// Internal lowering error.
    #[error(code = "EM900", message = "internal error: {message}")]
    Internal {
        /// Anchor the error to a module.
        module: ModuleId,
        /// Describe the internal failure.
        message: String,
    },
}
