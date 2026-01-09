use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_mir::AnchoredGlobalNodeId;
use destack_workspace::Program;

/// Warnings during the link phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Link)]
pub enum LinkWarning {
    // -------------------------------------------------------------------------
    // 1xx: Symbol issues
    // -------------------------------------------------------------------------
    /// Missing target for a symbol.
    #[warning(code = "WK100", message = "missing target for a symbol")]
    MissingTarget { node: AnchoredGlobalNodeId },

    /// Weak/duplicate symbol but one chosen deterministically (e.g. ODR violation that's survivable).
    #[warning(code = "WK101", message = "weak symbol")]
    WeakSymbol {
        node: AnchoredGlobalNodeId,
        symbol: String,
    },

    // -------------------------------------------------------------------------
    // 2xx: Size issues
    // -------------------------------------------------------------------------
    /// Large binary / large static data section.
    #[warning(code = "WK200", message = "large binary")]
    LargeBinary {
        node: AnchoredGlobalNodeId,
        size_mb: u64,
    },
}
