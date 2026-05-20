use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Errors during the import phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Import)]
pub enum ImportError {
    /// Unresolved local module.
    #[diagnostic(code = "EI200", message = "unresolved module '{target}'")]
    UnresolvedModule {
        anchor: DiagnosticAnchor,
        target: String,
    },

    /// Invalid loader type in import attributes.
    #[diagnostic(code = "EI203", message = "invalid import attribute type '{value}'")]
    InvalidImportAttributeType {
        anchor: DiagnosticAnchor,
        value: String,
    },

    /// Module specifier is outside the supported Destack import model.
    #[diagnostic(code = "EI204", message = "unsupported module specifier '{target}'")]
    UnsupportedModuleSpecifier {
        anchor: DiagnosticAnchor,
        target: String,
    },

    /// Extensionless local module specifier resolved to multiple modules.
    #[diagnostic(
        code = "EI205",
        message = "ambiguous module specifier '{target}': {candidates}"
    )]
    AmbiguousModuleSpecifier {
        anchor: DiagnosticAnchor,
        target: String,
        candidates: String,
    },

    /// Local module specifier resolves outside the importing package.
    #[diagnostic(
        code = "EI206",
        message = "local module specifier '{target}' crosses package boundaries"
    )]
    CrossPackageImport {
        anchor: DiagnosticAnchor,
        target: String,
    },

    /// Internal import failure.
    #[diagnostic(code = "EI900", message = "internal error: {message}")]
    Internal {
        anchor: DiagnosticAnchor,
        message: String,
    },
}
