use destack_artifact_macros::Diagnostic;

/// Warnings during the check phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Check)]
pub enum CheckWarning {}
