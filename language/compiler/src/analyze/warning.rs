use tspp_artifact_macros::Diagnostic;

/// Warnings during the analyze phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Analyze)]
pub enum AnalyzeWarning {}
