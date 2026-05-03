use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the link phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Link)]
pub enum LinkWarning {
    // -------------------------------------------------------------------------
    // 1xx: Symbol issues
    // -------------------------------------------------------------------------
    /// Missing target for a symbol.
    #[diagnostic(code = "WK100", message = "missing target for a symbol")]
    MissingTarget { anchor: DiagnosticAnchor },

    /// Weak/duplicate symbol but one chosen deterministically (e.g. ODR violation that's survivable).
    #[diagnostic(code = "WK101", message = "weak symbol")]
    WeakSymbol {
        anchor: DiagnosticAnchor,
        symbol: String,
    },

    // -------------------------------------------------------------------------
    // 2xx: Size issues
    // -------------------------------------------------------------------------
    /// Large binary / large static data section.
    #[diagnostic(code = "WK200", message = "large binary")]
    LargeBinary {
        anchor: DiagnosticAnchor,
        size_mb: u64,
    },
}
