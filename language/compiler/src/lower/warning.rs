use tspp_artifact_macros::Diagnostic;

/// Warnings during the lower phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Lower)]
pub enum LowerWarning {}
