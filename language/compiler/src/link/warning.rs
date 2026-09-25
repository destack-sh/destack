use crate::DiagnosticAnchor;
use tspp_artifact_macros::Diagnostic;

/// Warnings during the link phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Link)]
pub enum LinkWarning {
    // -------------------------------------------------------------------------
    // symbols
    // -------------------------------------------------------------------------
    /// Missing target for a symbol.
    #[diagnostic(id = "missing-symbol-target", message = "missing target for a symbol")]
    MissingTarget { anchor: DiagnosticAnchor },

    /// Weak/duplicate symbol but one chosen deterministically (e.g. ODR violation that's survivable).
    #[diagnostic(id = "weak-symbol", message = "weak symbol")]
    WeakSymbol {
        anchor: DiagnosticAnchor,
        symbol: String,
    },

    // -------------------------------------------------------------------------
    // size
    // -------------------------------------------------------------------------
    /// Large binary / large static data section.
    #[diagnostic(id = "large-binary", message = "large binary")]
    LargeBinary {
        anchor: DiagnosticAnchor,
        size_mb: u64,
    },
}
