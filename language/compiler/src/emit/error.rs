use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Errors during the emit phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Emit)]
pub enum EmitError {
    // -------------------------------------------------------------------------
    // target and setup
    // -------------------------------------------------------------------------
    /// Unsupported target/output format.
    #[diagnostic(id = "unsupported-target", message = "unsupported target: {target}")]
    UnsupportedTarget {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        target: String,
    },

    /// Unresolved function reference.
    #[diagnostic(id = "unresolved-function", message = "unresolved function: {name}")]
    UnresolvedFunction {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        name: String,
    },

    /// Native code generation is unavailable.
    #[diagnostic(
        id = "native-emit-unavailable",
        message = "native code generation is unavailable"
    )]
    NativeEmitUnavailable {
        anchor: DiagnosticAnchor,
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // types
    // -------------------------------------------------------------------------
    /// Unsupported type for emit.
    #[diagnostic(id = "unsupported-emission-type", message = "unsupported type")]
    UnsupportedType {
        anchor: DiagnosticAnchor,
        module: ModuleId,
    },

    /// Missing type information.
    #[diagnostic(id = "missing-type-information", message = "missing type")]
    MissingType {
        anchor: DiagnosticAnchor,
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // constructs
    // -------------------------------------------------------------------------
    /// Unsupported construct (instruction, expression, etc.).
    #[diagnostic(
        id = "unsupported-emission-construct",
        message = "unsupported construct: {message}"
    )]
    UnsupportedConstruct {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        message: String,
    },

    /// Unexpected construct (wrong node type).
    #[diagnostic(
        id = "unexpected-emission-construct",
        message = "unexpected construct: {message}"
    )]
    UnexpectedConstruct {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        message: String,
    },

    /// Unresolved construct.
    #[diagnostic(id = "unresolved-construct", message = "unresolved construct")]
    UnresolvedConstruct {
        anchor: DiagnosticAnchor,
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // bounds and access
    // -------------------------------------------------------------------------
    /// Out of bounds access (tuple/array element index).
    #[diagnostic(
        id = "out-of-bounds",
        message = "index {index} out of bounds (len {len})"
    )]
    OutOfBounds {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        index: u32,
        len: usize,
    },

    // -------------------------------------------------------------------------
    // internal failures
    // -------------------------------------------------------------------------
    /// Internal emit error.
    #[diagnostic(id = "internal-emission-error", message = "internal error: {message}")]
    Internal {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        message: String,
    },
}
