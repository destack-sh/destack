use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Warnings during the bind phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Bind)]
pub enum BindWarning {
    /// Use of deprecated target / CPU / ABI.
    #[warning(code = "WB001", message = "deprecated target")]
    DeprecatedTarget { node: GlobalNodeIdAny },

    /// Weak/duplicate symbol but one chosen deterministically (e.g. ODR violation that's survivable).
    #[warning(code = "WB002", message = "weak symbol")]
    WeakSymbol {
        node: GlobalNodeIdAny,
        symbol: String,
    },

    /// Large binary / large static data section ("binary size exceeded X MB").
    #[warning(code = "WB003", message = "large binary")]
    LargeBinary { node: GlobalNodeIdAny, size_mb: u64 },
}
