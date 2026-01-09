use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::GlobalNodeIdAny;
use destack_source::ModuleId;
use destack_workspace::Program;

/// Warnings during the import phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Import)]
pub enum ImportWarning {
    /// Huge file.
    #[warning(code = "WI001", message = "oversized file ({len} bytes)")]
    OversizedFile { module: ModuleId, len: usize },

    /// Use of deprecated target / CPU / ABI.
    #[warning(code = "WI002", message = "deprecated target")]
    DeprecatedTarget { node: GlobalNodeIdAny },

    /// Weak/duplicate symbol but one chosen deterministically (e.g. ODR violation that's survivable).
    #[warning(code = "WI003", message = "weak symbol")]
    WeakSymbol {
        node: GlobalNodeIdAny,
        symbol: String,
    },

    /// Large binary / large static data section ("binary size exceeded X MB").
    #[warning(code = "WI004", message = "large binary")]
    LargeBinary { node: GlobalNodeIdAny, size_mb: u64 },
}
