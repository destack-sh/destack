use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir as dir;
use destack_workspace::Program;
use dir::AnchoredGlobalNodeId;

/// Warnings during the elaborate phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Elaborate)]
pub enum ElaborateWarning {
    // -------------------------------------------------------------------------
    // 1xx: Configuration
    // -------------------------------------------------------------------------
    /// Implicit collection conversions are enabled with warnings.
    #[warning(code = "WE100", message = "implicit collection conversion")]
    ImplicitCollectionConversion { node: AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported node.
    #[warning(code = "WE900", message = "unsupported construct")]
    UnsupportedConstruct { node: AnchoredGlobalNodeId },
}
