use destack_artifact_macros::Diagnostic;

/// Warnings during the emit phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Emit)]
pub enum EmitWarning {}
