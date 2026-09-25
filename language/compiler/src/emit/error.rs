use crate::DiagnosticAnchor;
use tspp_artifact::Code;
use tspp_artifact_macros::Diagnostic;
use tspp_source::ModuleId;

/// Errors during the emit phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Emit)]
pub enum EmitError {
    // -------------------------------------------------------------------------
    // target and setup
    // -------------------------------------------------------------------------
    /// Unsupported target output.
    #[diagnostic(id = "unsupported-target", message = "unsupported target: {target}")]
    UnsupportedTarget {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        target: String,
    },

    /// Selected code has no emitter in this compiler build.
    #[diagnostic(id = "missing-emitter", message = "missing {code} emitter")]
    MissingEmitter {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        code: Code,
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
