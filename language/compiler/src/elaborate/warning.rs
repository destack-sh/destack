use tspp_artifact_macros::Diagnostic;

/// Warnings during the elaborate phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Elaborate)]
pub enum ElaborateWarning {}
