use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::{PackageId, TargetId};

/// Errors during the link phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Link)]
pub enum LinkError {
    // -------------------------------------------------------------------------
    // 1xx: Target issues
    // -------------------------------------------------------------------------
    /// Missing target.
    #[diagnostic(code = "EK100", message = "missing target: {target}")]
    MissingTarget {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
    },

    /// Invalid target configuration.
    #[diagnostic(code = "EK101", message = "invalid target: {target}: {message}")]
    InvalidTarget {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        message: String,
    },

    /// Invalid module kind for one linked subject.
    #[diagnostic(
        code = "EK102",
        message = "invalid module kind: {target}: {subject} expected {expected}, found '{found}'"
    )]
    InvalidModuleKind {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        subject: String,
        expected: String,
        found: String,
    },

    /// Unsupported suffix on one linked reference.
    #[diagnostic(
        code = "EK103",
        message = "unsupported reference suffix: {target}: {reference} does not support query or fragment suffix yet: '{value}'"
    )]
    UnsupportedReferenceSuffix {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        reference: String,
        value: String,
    },

    /// Invalid output path state for one linked subject.
    #[diagnostic(
        code = "EK104",
        message = "invalid output path: {target}: {subject} has no usable emitted path segment from '{value}'"
    )]
    InvalidOutputPath {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        subject: String,
        value: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Internal
    // -------------------------------------------------------------------------
    /// Internal error during linking.
    #[diagnostic(code = "EK900", message = "internal error: {message}")]
    Internal {
        anchor: DiagnosticAnchor,
        package: PackageId,
        message: String,
    },
}
