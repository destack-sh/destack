use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Errors during the export phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Export)]
pub enum ExportError {
    /// Export clause references a local binding that is not declared.
    ///
    /// ```ds
    /// export { missing };
    /// ```
    #[diagnostic(code = "ET100", message = "missing exported local binding '{name}'")]
    MissingExportBinding {
        /// The export clause.
        anchor: DiagnosticAnchor,
        /// The missing local binding name.
        name: String,
    },

    /// Module exports the same key twice.
    ///
    /// ```ds
    /// export const value = 1;
    /// export { value as value };
    /// ```
    #[diagnostic(code = "ET101", message = "duplicate export '{key}'")]
    DuplicateExport {
        /// The duplicate export declaration.
        anchor: DiagnosticAnchor,
        /// The duplicate export key.
        key: String,
    },

    /// Global export uses the default export key, which has no ambient name.
    ///
    /// ```ds
    /// global {
    ///     export { value as default };
    /// }
    /// ```
    #[diagnostic(code = "ET102", message = "global export cannot use default key")]
    DefaultGlobalExport {
        /// The default global export item.
        anchor: DiagnosticAnchor,
    },

    /// Global export uses a bare namespace selector, which has no ambient name.
    ///
    /// ```ds
    /// global {
    ///     export * from "./module.ds";
    /// }
    /// ```
    #[diagnostic(code = "ET107", message = "global namespace export requires an alias")]
    NamespaceGlobalExport {
        /// The bare namespace re-export item.
        anchor: DiagnosticAnchor,
    },

    /// Static export guard has no condition argument.
    ///
    /// ```ds
    /// @if
    /// export { debug };
    /// ```
    #[diagnostic(code = "ET103", message = "`@if` export guard requires a condition")]
    StaticIfRequiresCondition {
        /// The `@if` decorator.
        anchor: DiagnosticAnchor,
    },

    /// Static export guard has more than one condition argument.
    ///
    /// ```ds
    /// @if(import.meta.profile == "test", true)
    /// export { debug };
    /// ```
    #[diagnostic(
        code = "ET104",
        message = "`@if` export guard requires exactly one condition"
    )]
    StaticIfRequiresOneArgument {
        /// The `@if` decorator.
        anchor: DiagnosticAnchor,
    },

    /// Static export guard evaluates to a non-boolean value.
    ///
    /// ```ds
    /// @if(import.meta.profile)
    /// export { debug };
    /// ```
    #[diagnostic(
        code = "ET105",
        message = "`@if` export guard condition must be boolean"
    )]
    StaticIfRequiresBoolean {
        /// The `@if` condition.
        anchor: DiagnosticAnchor,
    },

    /// Static export guard depends on a value that is not available during export.
    ///
    /// ```ds
    /// @if(T extends string)
    /// export { debug };
    /// ```
    #[diagnostic(code = "ET106", message = "`@if` export guard condition is not static")]
    StaticIfNotStatic {
        /// The non-static `@if` condition.
        anchor: DiagnosticAnchor,
    },

    /// Internal export failure.
    #[diagnostic(code = "ET900", message = "internal error: {message}")]
    Internal {
        /// The source that triggered the internal failure.
        anchor: DiagnosticAnchor,
        /// The module being exported.
        module: ModuleId,
        /// The internal failure message.
        message: String,
    },
}
