use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Warnings during the link phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Link)]
pub enum LinkWarning {
    /// Missing target for a symbol.
    #[warning(code = "WK001", message = "missing target for a symbol")]
    MissingTarget { node: GlobalNodeIdAny },

    /// Weak/duplicate symbol but one chosen deterministically (e.g. ODR violation that's survivable).
    #[warning(code = "WK002", message = "weak symbol")]
    WeakSymbol {
        node: GlobalNodeIdAny,
        symbol: String,
    },

    /// Large binary / large static data section.
    #[warning(code = "WK003", message = "large binary")]
    LargeBinary { node: GlobalNodeIdAny, size_mb: u64 },
}
