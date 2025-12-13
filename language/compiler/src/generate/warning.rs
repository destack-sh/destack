use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Warnings during the generate phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Generate)]
pub enum GenerateWarning {
    /// Imprecise type (loss of precision in codegen).
    #[warning(code = "WG001", message = "imprecise type")]
    ImpreciseType { node: GlobalNodeIdAny },

    /// Unexpected construct (recoverable).
    #[warning(code = "WG002", message = "unexpected construct")]
    UnexpectedConstruct { node: GlobalNodeIdAny },
}
