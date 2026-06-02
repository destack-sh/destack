use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Errors during the import phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Import)]
pub enum ImportError {
    /// Local module specifier does not resolve to a source module.
    ///
    /// ```ds
    /// import { User } from "./missing.ds";
    /// ```
    #[diagnostic(code = "EI200", message = "unresolved module '{target}'")]
    UnresolvedModule {
        /// The import or export specifier.
        anchor: DiagnosticAnchor,
        /// The unresolved module specifier.
        target: String,
    },

    /// Import attribute value is not a supported loader name.
    ///
    /// ```ds
    /// import data from "./data.json" with { type: "binary" };
    /// ```
    #[diagnostic(code = "EI203", message = "invalid import attribute type '{value}'")]
    InvalidImportAttributeType {
        /// The invalid import attribute.
        anchor: DiagnosticAnchor,
        /// The unsupported attribute value.
        value: String,
    },

    /// Module specifier is outside the supported Destack import model.
    ///
    /// ```ds
    /// import { value } from "https://example.com/value.ds";
    /// ```
    #[diagnostic(code = "EI204", message = "unsupported module specifier '{target}'")]
    UnsupportedModuleSpecifier {
        /// The unsupported specifier.
        anchor: DiagnosticAnchor,
        /// The unsupported module specifier.
        target: String,
    },

    /// Extensionless local module specifier resolves to multiple source modules.
    ///
    /// ```ds
    /// import { value } from "./config";
    /// ```
    #[diagnostic(
        code = "EI205",
        message = "ambiguous module specifier '{target}': {candidates}"
    )]
    AmbiguousModuleSpecifier {
        /// The ambiguous specifier.
        anchor: DiagnosticAnchor,
        /// The ambiguous module specifier.
        target: String,
        /// The candidate modules.
        candidates: String,
    },

    /// Relative module specifier resolves outside the importing package.
    ///
    /// ```ds
    /// import { value } from "../other-package/value.ds";
    /// ```
    #[diagnostic(
        code = "EI206",
        message = "relative module specifier '{target}' crosses package boundaries"
    )]
    CrossPackageRelativeImport {
        /// The relative specifier.
        anchor: DiagnosticAnchor,
        /// The package-crossing module specifier.
        target: String,
    },

    /// Package export target resolves outside the exporting package.
    ///
    /// ```json
    /// { "exports": { ".": "../other-package/mod.ds" } }
    /// ```
    #[diagnostic(
        code = "EI207",
        message = "package export path '{target}' crosses package boundaries"
    )]
    CrossPackageExport {
        /// The package specifier.
        anchor: DiagnosticAnchor,
        /// The package export target.
        target: String,
    },

    /// Package specifier names a package that is not declared as a dependency.
    ///
    /// ```ds
    /// import { value } from "missing-package";
    /// ```
    #[diagnostic(
        code = "EI208",
        message = "package '{package}' is not declared as a dependency"
    )]
    MissingPackageDependency {
        /// The package specifier.
        anchor: DiagnosticAnchor,
        /// The missing package name.
        package: String,
    },

    /// Package specifier selects an export that is not active for this profile.
    ///
    /// ```ds
    /// import { value } from "library/missing";
    /// ```
    #[diagnostic(
        code = "EI209",
        message = "package '{package}' has no active export '{export}'"
    )]
    MissingPackageExport {
        /// The package specifier.
        anchor: DiagnosticAnchor,
        /// The package name.
        package: String,
        /// The missing export key.
        export: String,
    },

    /// Package export exists but does not point to a source module.
    ///
    /// ```ds
    /// import asset from "library/style.css";
    /// ```
    #[diagnostic(
        code = "EI210",
        message = "package '{package}' export '{export}' is not a module"
    )]
    NonModulePackageExport {
        /// The package specifier.
        anchor: DiagnosticAnchor,
        /// The package name.
        package: String,
        /// The non-module export key.
        export: String,
    },

    /// Package dependency is declared but not loaded in the source graph.
    ///
    /// ```ds
    /// import { value } from "library";
    /// ```
    #[diagnostic(
        code = "EI211",
        message = "package '{package}' dependency is not loaded"
    )]
    UnloadedPackageDependency {
        /// The package specifier.
        anchor: DiagnosticAnchor,
        /// The unloaded package name.
        package: String,
    },

    /// Static import guard has no condition argument.
    ///
    /// ```ds
    /// @if
    /// import { debug } from "./debug.ds";
    /// ```
    #[diagnostic(code = "EI212", message = "`@if` import guard requires a condition")]
    StaticIfRequiresCondition {
        /// The `@if` decorator.
        anchor: DiagnosticAnchor,
    },

    /// Static import guard has more than one condition argument.
    ///
    /// ```ds
    /// @if(import.meta.profile == "test", true)
    /// import { debug } from "./debug.ds";
    /// ```
    #[diagnostic(
        code = "EI213",
        message = "`@if` import guard requires exactly one condition"
    )]
    StaticIfRequiresOneArgument {
        /// The `@if` decorator.
        anchor: DiagnosticAnchor,
    },

    /// Static import guard evaluates to a non-boolean value.
    ///
    /// ```ds
    /// @if(import.meta.profile)
    /// import { debug } from "./debug.ds";
    /// ```
    #[diagnostic(
        code = "EI214",
        message = "`@if` import guard condition must be boolean"
    )]
    StaticIfRequiresBoolean {
        /// The `@if` condition.
        anchor: DiagnosticAnchor,
    },

    /// Static import guard depends on a value that is not available during import.
    ///
    /// ```ds
    /// @if(T extends string)
    /// import { debug } from "./debug.ds";
    /// ```
    #[diagnostic(code = "EI215", message = "`@if` import guard condition is not static")]
    StaticIfNotStatic {
        /// The non-static `@if` condition.
        anchor: DiagnosticAnchor,
    },

    /// Internal import failure.
    #[diagnostic(code = "EI900", message = "internal error: {message}")]
    Internal {
        /// The source that triggered the internal failure.
        anchor: DiagnosticAnchor,
        /// The internal failure message.
        message: String,
    },
}
