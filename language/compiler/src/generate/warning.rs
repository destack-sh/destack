use crate::{CompileWarning, DiagnosticAnchor, DiagnosticDefinition};
use destack_compiler_macros::DefineWarning;
use destack_dir::AnchoredGlobalNodeId;
use destack_source::ModuleId;
use destack_workspace::Repository;

/// Warnings during the generate phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Generate)]
pub enum GenerateWarning {
    // -------------------------------------------------------------------------
    // 1xx: Type warnings
    // -------------------------------------------------------------------------
    /// Imprecise type (loss of precision in codegen).
    #[warning(code = "WG100", message = "imprecise type")]
    ImpreciseType {
        module: ModuleId,
        node: Option<AnchoredGlobalNodeId>,
    },

    // -------------------------------------------------------------------------
    // 2xx: Construct warnings
    // -------------------------------------------------------------------------
    /// Unexpected construct (recoverable).
    #[warning(code = "WG200", message = "unexpected construct")]
    UnexpectedConstruct {
        module: ModuleId,
        node: Option<AnchoredGlobalNodeId>,
    },
}
