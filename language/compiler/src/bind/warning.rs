use destack_artifact_macros::Diagnostic;

/// Warnings during source binding.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Bind)]
pub enum BindWarning {}
