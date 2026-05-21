use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Errors during the export phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Export)]
pub enum ExportError {
    /// Export clause references a local binding that is not declared.
    #[diagnostic(code = "ET100", message = "missing exported local binding '{name}'")]
    MissingExportBinding {
        anchor: DiagnosticAnchor,
        name: String,
    },

    /// Module exports the same key twice.
    #[diagnostic(code = "ET101", message = "duplicate export '{key}'")]
    DuplicateExport {
        anchor: DiagnosticAnchor,
        key: String,
    },

    /// Global export uses a form that cannot contribute an ambient name.
    #[diagnostic(code = "ET102", message = "unsupported global export")]
    UnsupportedGlobalExport { anchor: DiagnosticAnchor },

    /// Internal export failure.
    #[diagnostic(code = "ET900", message = "internal error: {message}")]
    Internal {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        message: String,
    },
}
