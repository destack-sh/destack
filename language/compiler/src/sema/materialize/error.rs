use crate::{DiagnosticAnchor, LowerError};
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Errors during the materialize phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Materialize)]
pub enum MaterializeError {
    // -------------------------------------------------------------------------
    // compile-time execution
    // -------------------------------------------------------------------------
    /// Comptime lowering failed.
    #[diagnostic(
        id = "comptime-lowering-failed",
        message = "comptime lowering failed: {message}"
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

    /// Comptime execution failed.
    #[diagnostic(
        id = "comptime-execution-failed",
        message = "comptime execution failed: {message}"
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
    /// Unsupported construct for comptime execution.
    #[diagnostic(
        id = "unsupported-comptime-construct",
        message = "unsupported construct for comptime execution"
    )]
    UnsupportedConstruct { anchor: DiagnosticAnchor },
}
