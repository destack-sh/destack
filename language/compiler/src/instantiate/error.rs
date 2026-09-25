use tspp_artifact_macros::Diagnostic;

/// Errors during the instantiate phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Instantiate)]
pub enum InstantiateError {}
