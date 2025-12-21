use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Warnings during the lower phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Lower)]
pub enum LowerWarning {
    /// Complex type in target language.
    #[warning(code = "WM001", message = "complex type in target")]
    ComplexType {
        /// Point at the node that introduced a complex type.
        node: GlobalNodeIdAny,
    },

    /// This feature will be emulated slowly on this target.
    #[warning(code = "WM002", message = "slow emulation in target: '{feature}'")]
    SlowEmulation {
        /// Point at the node that triggered slow emulation.
        node: GlobalNodeIdAny,
        /// Describe the feature that will be emulated.
        feature: String,
    },
}
