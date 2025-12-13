use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Warnings during the lower phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Lower)]
pub enum LowerWarning {
    /// Complex type in target language.
    #[warning(code = "WL001", message = "complex type in target")]
    ComplexType { node: GlobalNodeIdAny },

    /// This feature will be emulated slowly on this target.
    #[warning(code = "WL002", message = "slow emulation in target")]
    SlowEmulation {
        node: GlobalNodeIdAny,
        feature: String,
    },
}
