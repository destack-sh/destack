use tspp_artifact_macros::Diagnostic;

/// Errors during the analyze phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Analyze)]
pub enum AnalyzeError {}
