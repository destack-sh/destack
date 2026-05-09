use crate::{DiagnosticAnchor, LowerError};
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Errors during the materialize phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Materialize)]
pub enum MaterializeError {
    // -------------------------------------------------------------------------
    // 1xx: Comptime execution errors
    // -------------------------------------------------------------------------
    /// Comptime lowering failed.
    #[diagnostic(code = "EM100", message = "comptime lowering failed: {message}")]
    FailedLower {
        /// Anchor the error to a module.
        anchor: DiagnosticAnchor,
        /// Identify the module where execution failed.
        module: ModuleId,
        /// Report the lowering error.
        error: Box<LowerError>,
        /// Describe the lowering failure.
        message: String,
    },

    /// Comptime execution failed.
    #[diagnostic(code = "EM101", message = "comptime execution failed: {message}")]
    FailedExecution {
        /// Anchor the error to a module.
        anchor: DiagnosticAnchor,
        /// Identify the module where execution failed.
        module: ModuleId,
        /// Describe the execution failure.
        message: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported construct for comptime execution.
    #[diagnostic(
        code = "EM900",
        message = "unsupported construct for comptime execution"
    )]
    UnsupportedConstruct { anchor: DiagnosticAnchor },
}
