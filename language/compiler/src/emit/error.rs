use crate::DiagnosticAnchor;
use destack_artifact::Code;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

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
    /// Unsupported type during emission.
    #[diagnostic(id = "unsupported-emission-type", message = "unsupported type")]
    UnsupportedType {
        anchor: DiagnosticAnchor,
        module: ModuleId,
    },

    /// Unexpected construct during emission.
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
    /// Internal emission error.
    #[diagnostic(id = "internal-emission-error", message = "internal error: {message}")]
    Internal {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        message: String,
    },
}
