use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::{PackageId, TargetId};

/// Errors during the optimize phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Optimize)]
pub enum OptimizeError {
    /// Invalid target configuration for optimization.
    #[diagnostic(code = "EO110", message = "invalid target {target}: {message}")]
    InvalidTarget {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        message: String,
    },

    /// Unsupported MIR construct encountered during optimization.
    #[diagnostic(code = "EO900", message = "unsupported MIR construct")]
    UnsupportedConstruct { anchor: DiagnosticAnchor },

    /// Internal optimization error.
    #[diagnostic(code = "EO901", message = "internal optimization error: {message}")]
    InternalError {
        anchor: DiagnosticAnchor,
        message: String,
    },
}
