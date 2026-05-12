use destack_artifact_macros::Diagnostic;

/// Warnings during the verify phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Verify)]
pub enum VerifyWarning {}
