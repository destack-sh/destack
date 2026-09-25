use crate::{DiagnosticAnchor, LowerError};
use tspp_artifact_macros::Diagnostic;
use tspp_source::ModuleId;

/// Errors during the materialize phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Materialize)]
pub enum MaterializeError {
    // -------------------------------------------------------------------------
    // compile-time execution
    // -------------------------------------------------------------------------
    /// Const lowering failed.
    #[diagnostic(
        id = "const-lowering-failed",
        message = "const lowering failed: {message}"
    )]
    FailedLower {
        /// Anchor the error to a module.
        anchor: DiagnosticAnchor,
        /// Identify the module where execution failed.
        module: ModuleId,
        /// Preserve the lower phase diagnostic.
        error: Box<LowerError>,
        /// Describe the lowering failure.
        message: String,
    },

    /// Const execution failed.
    #[diagnostic(
        id = "const-execution-failed",
        message = "const execution failed: {message}"
    )]
    FailedExecution {
        /// Anchor the error to a module.
        anchor: DiagnosticAnchor,
        /// Identify the module where execution failed.
        module: ModuleId,
        /// Describe the execution failure.
        message: String,
    },

    // -------------------------------------------------------------------------
    // unsupported and internal failures
    // -------------------------------------------------------------------------
    /// Unsupported construct for const execution.
    #[diagnostic(
        id = "unsupported-const-construct",
        message = "unsupported construct for const execution"
    )]
    UnsupportedConstruct { anchor: DiagnosticAnchor },
}
