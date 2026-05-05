use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the resolve phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Resolve)]
pub enum ResolveWarning {
    // -------------------------------------------------------------------------
    // 1xx: Import issues
    // -------------------------------------------------------------------------
    /// Unknown import.
    #[diagnostic(code = "WR100", message = "unknown import")]
    UnknownImport { anchor: DiagnosticAnchor },

    /// Unused imports or unused re-exports.
    #[diagnostic(code = "WR101", message = "unused import")]
    UnusedImport { anchor: DiagnosticAnchor },

    /// Import that resolves but is only used for side effects.
    #[diagnostic(code = "WR102", message = "side effect only import")]
    SideEffectOnlyImport { anchor: DiagnosticAnchor },

    /// Unresolved module (for lenient resolve mode only).
    #[diagnostic(code = "WR103", message = "unresolved module '{target}'")]
    UnresolvedModule {
        anchor: DiagnosticAnchor,
        target: String,
    },

    /// Unprefixed library module import resolved through compatibility canonicalization.
    #[diagnostic(
        code = "WR104",
        message = "unprefixed library module '{target}' resolved as '{suggested}'"
    )]
    UnprefixedBuiltinModule {
        anchor: DiagnosticAnchor,
        target: String,
        suggested: String,
    },
}
