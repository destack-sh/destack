use crate::DiagnosticAnchor;
use tspp_artifact_macros::Diagnostic;
use tspp_source::ModuleId;

/// Errors during the export phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Export)]
pub enum ExportError {
    /// Export clause references a local binding that is not declared.
    ///
    /// ```tspp
    /// export { missing };
    /// ```
    #[diagnostic(
        id = "missing-export-binding",
        message = "missing exported local binding '{name}'",
        optional_message = "; did you mean '{suggestion}'?"
    )]
    MissingExportBinding {
        /// The export clause.
        anchor: DiagnosticAnchor,
        /// The missing local binding name.
        name: String,
        /// The closest module-scope binding name.
        suggestion: Option<String>,
    },

    /// Module exports the same key twice.
    ///
    /// ```tspp
    /// export const value = 1;
    /// export { value as value };
    /// ```
    #[diagnostic(id = "duplicate-export", message = "duplicate export '{key}'")]
    DuplicateExport {
        /// The duplicate export declaration.
        anchor: DiagnosticAnchor,
        /// The duplicate export key.
        key: String,
    },

    /// Global export uses the default export key, which has no ambient name.
    ///
    /// ```tspp
    /// global {
    ///     export { value as default };
    /// }
    /// ```
    #[diagnostic(
        id = "default-global-export",
        message = "global export cannot use default key"
    )]
    DefaultGlobalExport {
        /// The default global export item.
        anchor: DiagnosticAnchor,
    },

    /// Global export uses a bare namespace selector, which has no ambient name.
    ///
    /// ```tspp
    /// global {
    ///     export * from "./module.tspp";
    /// }
    /// ```
    #[diagnostic(
        id = "namespace-global-export",
        message = "global namespace export requires an alias"
    )]
    NamespaceGlobalExport {
        /// The bare namespace re-export item.
        anchor: DiagnosticAnchor,
    },

    /// Static export guard has no condition argument.
    ///
    /// ```tspp
    /// @if
    /// export { debug };
    /// ```
    #[diagnostic(
        id = "missing-static-export-condition",
        message = "`@if` export guard requires a condition"
    )]
    StaticIfRequiresCondition {
        /// The `@if` decorator.
        anchor: DiagnosticAnchor,
    },

    /// Static export guard has more than one condition argument.
    ///
    /// ```tspp
    /// @if(import.meta.profile == "test", true)
    /// export { debug };
    /// ```
    #[diagnostic(
        id = "multiple-static-export-conditions",
        message = "`@if` export guard requires exactly one condition"
    )]
    StaticIfRequiresOneArgument {
        /// The `@if` decorator.
        anchor: DiagnosticAnchor,
    },

    /// Static export guard evaluates to a non-boolean value.
    ///
    /// ```tspp
    /// @if(import.meta.profile)
    /// export { debug };
    /// ```
    #[diagnostic(
        id = "non-boolean-static-export-condition",
        message = "`@if` export guard condition must be boolean"
    )]
    StaticIfRequiresBoolean {
        /// The `@if` condition.
        anchor: DiagnosticAnchor,
    },

    /// Static export guard depends on a value that is not available during export.
    ///
    /// ```tspp
    /// @if(enabled)
    /// export { debug };
    /// ```
    #[diagnostic(
        id = "non-static-export-condition",
        message = "`@if` export guard condition is not static"
    )]
    StaticIfNotStatic {
        /// The non-static `@if` condition.
        anchor: DiagnosticAnchor,
    },

    /// Static export guard is not invoked in its intrinsic form.
    ///
    /// ```tspp
    /// @if<boolean>(true)
    /// export { debug };
    /// ```
    #[diagnostic(
        id = "invalid-static-export-condition",
        message = "`@if` export guard must be invoked as `@if(condition)`"
    )]
    InvalidStaticIfInvocation {
        /// The malformed `@if` decorator.
        anchor: DiagnosticAnchor,
    },

    /// Internal export failure.
    #[diagnostic(id = "internal-export-error", message = "internal error: {message}")]
    Internal {
        /// The source that triggered the internal failure.
        anchor: DiagnosticAnchor,
        /// The module being exported.
        module: ModuleId,
        /// The internal failure message.
        message: String,
    },
}
