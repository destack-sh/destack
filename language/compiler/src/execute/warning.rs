use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Warnings during the execute phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Execute)]
pub enum ExecuteWarning {
    /// Unsupported construct.
    #[warning(code = "WX001", message = "unsupported construct")]
    UnsupportedConstruct { node: GlobalNodeIdAny },
}
