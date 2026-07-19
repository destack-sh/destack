use destack_artifact_macros::Diagnostic;

/// Warnings during the resolve phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Resolve)]
pub enum ResolveWarning {}
