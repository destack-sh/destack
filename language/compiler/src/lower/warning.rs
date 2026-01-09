use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::AnchoredGlobalNodeId;
use destack_workspace::Program;

/// Warnings during the lower phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Lower)]
pub enum LowerWarning {
    // -------------------------------------------------------------------------
    // 1xx: Type warnings
    // -------------------------------------------------------------------------
    /// Complex type in target language.
    #[warning(code = "WM100", message = "complex type in target")]
    ComplexType {
        /// Point at the node that introduced a complex type.
        node: AnchoredGlobalNodeId,
    },
}
