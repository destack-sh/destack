use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::AnchoredGlobalNodeId;
use destack_workspace::Program;

/// Warnings during the execute phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Execute)]
pub enum ExecuteWarning {
    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported construct.
    #[warning(code = "WX900", message = "unsupported construct")]
    UnsupportedConstruct { node: AnchoredGlobalNodeId },
}
