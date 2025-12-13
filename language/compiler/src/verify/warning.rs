use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Warnings during the verify phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Verify)]
pub enum VerifyWarning {
    /// Unsupported node.
    #[warning(code = "WV001", message = "unsupported construct")]
    UnsupportedConstruct { node: GlobalNodeIdAny },
}
