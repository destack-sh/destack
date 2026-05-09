use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the import phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Import)]
pub enum ImportWarning {
    // -------------------------------------------------------------------------
    // 1xx: Import issues
    // -------------------------------------------------------------------------
    /// Unknown import.
    #[diagnostic(code = "WI100", message = "unknown import")]
    UnknownImport { anchor: DiagnosticAnchor },

    /// Unused imports or unused re-exports.
    #[diagnostic(code = "WI101", message = "unused import")]
    UnusedImport { anchor: DiagnosticAnchor },

    /// Import that resolves but is only used for side effects.
    #[diagnostic(code = "WI102", message = "side effect only import")]
    SideEffectOnlyImport { anchor: DiagnosticAnchor },

    /// Unresolved module (for lenient resolve mode only).
    #[diagnostic(code = "WI103", message = "unresolved module '{target}'")]
    UnresolvedModule {
        anchor: DiagnosticAnchor,
        target: String,
    },

    /// Unprefixed library module import resolved through compatibility canonicalization.
    #[diagnostic(
        code = "WI104",
        message = "unprefixed library module '{target}' resolved as '{suggested}'"
    )]
    UnprefixedBuiltinModule {
        anchor: DiagnosticAnchor,
        target: String,
        suggested: String,
    },
}
