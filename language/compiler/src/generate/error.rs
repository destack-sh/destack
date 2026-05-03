use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Errors during the generate phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Generate)]
pub enum GenerateError {
    // -------------------------------------------------------------------------
    // 1xx: Target / setup
    // -------------------------------------------------------------------------
    /// Unsupported target/output format.
    #[diagnostic(code = "EG100", message = "unsupported target: {target}")]
    UnsupportedTarget {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        target: String,
    },

    /// Unresolved function reference.
    #[diagnostic(code = "EG101", message = "unresolved function: {name}")]
    UnresolvedFunction {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        name: String,
    },

    // -------------------------------------------------------------------------
    // 2xx: Type issues
    // -------------------------------------------------------------------------
    /// Unsupported type for codegen.
    #[diagnostic(code = "EG200", message = "unsupported type")]
    UnsupportedType {
        anchor: DiagnosticAnchor,
        module: ModuleId,
    },

    /// Missing type information.
    #[diagnostic(code = "EG201", message = "missing type")]
    MissingType {
        anchor: DiagnosticAnchor,
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 3xx: Construct issues
    // -------------------------------------------------------------------------
    /// Unsupported construct (instruction, expression, etc.).
    #[diagnostic(code = "EG300", message = "unsupported construct: {message}")]
    UnsupportedConstruct {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        message: String,
    },

    /// Unexpected construct (wrong node type).
    #[diagnostic(code = "EG301", message = "unexpected construct: {message}")]
    UnexpectedConstruct {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        message: String,
    },

    /// Unresolved construct (not fully resolved before codegen).
    #[diagnostic(code = "EG302", message = "unresolved construct")]
    UnresolvedConstruct {
        anchor: DiagnosticAnchor,
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 4xx: Bounds / access
    // -------------------------------------------------------------------------
    /// Out of bounds access (tuple/array element index).
    #[diagnostic(code = "EG400", message = "index {index} out of bounds (len {len})")]
    OutOfBounds {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        index: u32,
        len: usize,
    },

    // -------------------------------------------------------------------------
    // 9xx: Internal
    // -------------------------------------------------------------------------
    /// Internal codegen error.
    #[diagnostic(code = "EG900", message = "internal error: {message}")]
    Internal {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        message: String,
    },
}
