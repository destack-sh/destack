use destack_artifact_macros::Diagnostic;

/// Errors during source binding.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Bind)]
pub enum BindError {}
