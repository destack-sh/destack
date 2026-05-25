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

    /// Static export guard is missing a condition.
    #[diagnostic(code = "ET103", message = "`@if` export guard requires a condition")]
    StaticIfRequiresCondition { anchor: DiagnosticAnchor },

    /// Static export guard has more than one condition argument.
    #[diagnostic(
        code = "ET104",
        message = "`@if` export guard requires exactly one condition"
    )]
    StaticIfRequiresOneArgument { anchor: DiagnosticAnchor },

    /// Static export guard did not evaluate to a boolean.
    #[diagnostic(
        code = "ET105",
        message = "`@if` export guard condition must be boolean"
    )]
    StaticIfRequiresBoolean { anchor: DiagnosticAnchor },

    /// Static export guard uses a condition that cannot be evaluated here.
    #[diagnostic(code = "ET106", message = "`@if` export guard condition is not static")]
    StaticIfNotStatic { anchor: DiagnosticAnchor },

    /// Internal export failure.
    #[diagnostic(code = "ET900", message = "internal error: {message}")]
    Internal {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        message: String,
    },
}
