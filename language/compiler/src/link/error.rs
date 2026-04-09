use crate::{
    CompileError, DiagnosticAnchor, DiagnosticDefinition, RequirementError, RequirementSet,
};
use destack_compiler_macros::DefineError;
use destack_source::{PackageId, TargetId};
use destack_workspace::Repository;

/// Errors during the link phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Link)]
pub enum LinkError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / requirement
    // -------------------------------------------------------------------------
    /// Wait for artifact requirement.
    #[error(code = "EK000", r#yield)]
    Yield { requirement: RequirementSet },

    /// Yield requirement has failed.
    #[error(code = "EK001", yield_failed)]
    UnsatisfiedRequirement { requirement: RequirementSet },

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
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        message: String,
    },

    /// Invalid module kind for one linked subject.
    #[error(
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
    #[error(
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

    /// Missing graph edge for one linked reference.
    #[error(
        code = "EK104",
        message = "missing reference edge: {target}: {reference} missing graph edge for module '{module}' site {site} and specifier '{specifier}'"
    )]
    MissingReferenceEdge {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        reference: String,
        module: String,
        site: u32,
        specifier: String,
    },

    /// Invalid output path state for one linked subject.
    #[error(
        code = "EK105",
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
    #[error(code = "EK900", message = "internal error: {message}")]
    Internal { package: PackageId, message: String },
}
