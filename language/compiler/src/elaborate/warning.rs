use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Warnings during the elaborate phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Elaborate)]
pub enum ElaborateWarning {
    /// Unsupported node.
    #[warning(code = "WE001", message = "unsupported construct")]
    UnsupportedConstruct { node: GlobalNodeIdAny },
}
