use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Warnings during source declaration.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Declare)]
pub enum DeclareWarning {
    // -------------------------------------------------------------------------
    // 1xx: File / size warnings
    // -------------------------------------------------------------------------
    /// Huge file.
    #[diagnostic(code = "WD100", message = "oversized file ({len} bytes)")]
    OversizedFile {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        len: usize,
    },

    // -------------------------------------------------------------------------
    // 2xx: Target / config warnings
    // -------------------------------------------------------------------------
    /// Use of deprecated target / CPU / ABI.
    #[diagnostic(code = "WD200", message = "deprecated target")]
    DeprecatedTarget { anchor: DiagnosticAnchor },

    // -------------------------------------------------------------------------
    // 3xx: Symbol warnings
    // -------------------------------------------------------------------------
    /// Weak/duplicate symbol but one chosen deterministically (e.g. ODR violation that's survivable).
    #[diagnostic(code = "WD300", message = "weak symbol")]
    WeakSymbol {
        anchor: DiagnosticAnchor,
        symbol: String,
    },

    // -------------------------------------------------------------------------
    // 4xx: Size warnings
    // -------------------------------------------------------------------------
    /// Large binary / large static data section ("binary size exceeded X MB").
    #[diagnostic(code = "WD400", message = "large binary")]
    LargeBinary {
        anchor: DiagnosticAnchor,
        size_mb: u64,
    },
}
