use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::AnchoredGlobalNodeId;
use destack_workspace::Program;

/// Warnings during the resolve phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Resolve)]
pub enum ResolveWarning {
    // -------------------------------------------------------------------------
    // 1xx: Import issues
    // -------------------------------------------------------------------------
    /// Unknown import.
    #[warning(code = "WR100", message = "unknown import")]
    UnknownImport { node: AnchoredGlobalNodeId },

    /// Unused imports or unused re-exports.
    #[warning(code = "WR101", message = "unused import")]
    UnusedImport { node: AnchoredGlobalNodeId },

    /// Import that resolves but is only used for side effects.
    #[warning(code = "WR102", message = "side effect only import")]
    SideEffectOnlyImport { node: AnchoredGlobalNodeId },

    /// Unresolved module (for lenient resolve mode only).
    #[warning(code = "WR103", message = "unresolved module '{target}'")]
    UnresolvedModule {
        node: AnchoredGlobalNodeId,
        target: destack_dir::StringId,
    },
}
