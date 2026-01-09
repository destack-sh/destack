use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::AnchoredGlobalNodeId;
use destack_workspace::Program;

/// Warnings during the elaborate phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Elaborate)]
pub enum ElaborateWarning {
    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported node.
    #[warning(code = "WE900", message = "unsupported construct")]
    UnsupportedConstruct { node: AnchoredGlobalNodeId },
}
