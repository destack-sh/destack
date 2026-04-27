use crate::{CompileWarning, DiagnosticAnchor, DiagnosticDefinition};
use destack_compiler_macros::DefineWarning;
use destack_dir::AnchoredGlobalNodeId;
use destack_workspace::Repository;

/// Warnings during the analyze phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Analyze)]
pub enum AnalyzeWarning {
    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported node.
    #[warning(code = "WA900", message = "unsupported construct")]
    UnsupportedConstruct { node: AnchoredGlobalNodeId },
}
