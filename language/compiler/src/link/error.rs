use crate::{
    ArtifactRequirementError, ArtifactRequirementSet, DiagnosticAnchor, DiagnosticDefinition,
    TaskError,
};
use destack_compiler_macros::DefineError;
use destack_source::{PackageId, Span};
use destack_workspace::{Program, TargetId};

/// Errors during the link phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Link)]
pub enum LinkError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / requirement
    // -------------------------------------------------------------------------
    /// Wait for artifact requirement.
    #[error(code = "EK000", r#yield)]
    Yield { requirement: ArtifactRequirementSet },

    /// Yield requirement has failed.
    #[error(code = "EK001", yield_failed)]
    UnsatisfiedRequirement { requirement: ArtifactRequirementSet },

    /// Task was skipped due to stale versions.
    #[error(code = "EK002", message = "task skipped")]
    Skipped,

    // -------------------------------------------------------------------------
    // 1xx: Target issues
    // -------------------------------------------------------------------------
    /// Missing target.
    #[error(code = "EK100", message = "missing target: {target}")]
    MissingTarget {
        package: PackageId,
        target: TargetId,
    },

    /// Invalid target configuration.
    #[error(code = "EK101", message = "invalid target: {target}: {message}")]
    InvalidTarget {
        span: Span,
        package: PackageId,
        target: TargetId,
        message: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Internal
    // -------------------------------------------------------------------------
    /// Internal error during linking.
    #[error(code = "EK900", message = "internal error: {message}")]
    Internal { package: PackageId, message: String },
}
