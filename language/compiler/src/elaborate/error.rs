use tspp_artifact_macros::Diagnostic;

/// Errors during the elaborate phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Elaborate)]
pub enum ElaborateError {}
