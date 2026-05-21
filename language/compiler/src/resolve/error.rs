use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Errors during the resolve phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Resolve)]
pub enum ResolveError {
    /// Import or re-export selects a name that is not exported by the target module.
    #[diagnostic(code = "ER200", message = "missing export '{name}' from '{target}'")]
    MissingExport {
        anchor: DiagnosticAnchor,
        name: String,
        target: String,
    },

    /// Import or re-export selects a name that is re-exported by multiple star exports.
    #[diagnostic(code = "ER201", message = "ambiguous export '{name}' from '{target}'")]
    AmbiguousExport {
        anchor: DiagnosticAnchor,
        name: String,
        target: String,
    },

    /// Internal resolve failure.
    #[diagnostic(code = "ER900", message = "internal error: {message}")]
    Internal {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        message: String,
    },
}
