use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Warnings during the resolve phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Resolve)]
pub enum ResolveWarning {
    /// Unknown import.
    #[warning(code = "WR001", message = "unknown import")]
    UnknownImport { node: GlobalNodeIdAny },

    /// Unused imports or unused re-exports.
    #[warning(code = "WR002", message = "unused import")]
    UnusedImport { node: GlobalNodeIdAny },

    /// Import that resolves but is only used for side effects.
    #[warning(code = "WR003", message = "side effect only import")]
    SideEffectOnlyImport { node: GlobalNodeIdAny },
}
