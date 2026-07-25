use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::{PackageId, TargetId};

/// Errors during the optimize phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Optimize)]
pub enum OptimizeError {
    /// Invalid target configuration for optimization.
    #[diagnostic(
        id = "invalid-optimization-target",
        message = "invalid target {target}: {message}"
    )]
    InvalidTarget {
        /// The package containing the invalid target.
        anchor: DiagnosticAnchor,
        /// The package being optimized.
        package: PackageId,
        /// The invalid target.
        target: TargetId,
        /// The invalid target detail.
        message: String,
    },

    /// Unsupported MIR construct encountered during optimization.
    #[diagnostic(
        id = "unsupported-optimization-construct",
        message = "unsupported MIR construct"
    )]
    UnsupportedConstruct { anchor: DiagnosticAnchor },

    /// Internal optimization error.
    #[diagnostic(
        id = "internal-optimization-error",
        message = "internal optimization error: {message}"
    )]
    InternalError {
        anchor: DiagnosticAnchor,
        message: String,
    },
}
