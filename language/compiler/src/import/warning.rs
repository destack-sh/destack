use crate::{CompileWarning, DiagnosticAnchor, DiagnosticDefinition};
use destack_compiler_macros::DefineWarning;
use destack_dir::AnchoredGlobalNodeId;
use destack_source::ModuleId;
use destack_workspace::Repository;

/// Warnings during the import phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Import)]
pub enum ImportWarning {
    // -------------------------------------------------------------------------
    // 1xx: File / size warnings
    // -------------------------------------------------------------------------
    /// Huge file.
    #[warning(code = "WI100", message = "oversized file ({len} bytes)")]
    OversizedFile { module: ModuleId, len: usize },

    // -------------------------------------------------------------------------
    // 2xx: Target / config warnings
    // -------------------------------------------------------------------------
    /// Use of deprecated target / CPU / ABI.
    #[warning(code = "WI200", message = "deprecated target")]
    DeprecatedTarget { node: AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 3xx: Symbol warnings
    // -------------------------------------------------------------------------
    /// Weak/duplicate symbol but one chosen deterministically (e.g. ODR violation that's survivable).
    #[warning(code = "WI300", message = "weak symbol")]
    WeakSymbol {
        node: AnchoredGlobalNodeId,
        symbol: String,
    },

    // -------------------------------------------------------------------------
    // 4xx: Size warnings
    // -------------------------------------------------------------------------
    /// Large binary / large static data section ("binary size exceeded X MB").
    #[warning(code = "WI400", message = "large binary")]
    LargeBinary {
        node: AnchoredGlobalNodeId,
        size_mb: u64,
    },
}
