use crate::DiagnosticAnchor;
use tspp_artifact_macros::Diagnostic;

/// Errors during the import phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Import)]
pub enum ImportError {
    /// Local module specifier does not resolve to a source module.
    ///
    /// ```tspp
    /// import { User } from "./missing.tspp";
    /// ```
    #[diagnostic(
        id = "unresolved-module",
        message = "unresolved module '{target}'",
        optional_message = "; did you mean '{suggestion}'?"
    )]
    UnresolvedModule {
        /// The import or export specifier.
        anchor: DiagnosticAnchor,
        /// The unresolved module specifier.
        target: String,
        /// The closest visible module specifier.
        suggestion: Option<String>,
    },

    /// Import attribute value is not a supported loader name.
    ///
    /// ```tspp
    /// import data from "./data.json" with { type: "binary" };
    /// ```
    #[diagnostic(
        id = "invalid-import-attribute-type",
        message = "invalid import attribute type '{value}'",
        optional_message = "; did you mean '{suggestion}'?"
    )]
    InvalidImportAttributeType {
        /// The invalid import attribute.
        anchor: DiagnosticAnchor,
        /// The unsupported attribute value.
        value: String,
        /// The closest supported import attribute type.
        suggestion: Option<String>,
    },

    /// Module specifier is outside the supported TS++ import model.
    ///
    /// ```tspp
    /// import { value } from "https://example.com/value.tspp";
    /// ```
    #[diagnostic(
        id = "unsupported-module-specifier",
        message = "unsupported module specifier '{target}'"
    )]
    UnsupportedModuleSpecifier {
        /// The unsupported specifier.
        anchor: DiagnosticAnchor,
        /// The unsupported module specifier.
        target: String,
    },

    /// Extensionless local module specifier resolves to multiple source modules.
    ///
    /// ```tspp
    /// import { value } from "./config";
    /// ```
    #[diagnostic(
        id = "ambiguous-module-specifier",
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
    /// ```tspp
    /// import { value } from "../other-package/value.tspp";
    /// ```
    #[diagnostic(
        id = "cross-package-relative-import",
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
    /// { "exports": { ".": "../other-package/mod.tspp" } }
    /// ```
    #[diagnostic(
        id = "cross-package-export",
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
    /// ```tspp
    /// import { value } from "missing-package";
    /// ```
    #[diagnostic(
        id = "missing-package-dependency",
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
    /// ```tspp
    /// import { value } from "library/missing";
    /// ```
    #[diagnostic(
        id = "missing-package-export",
        message = "package '{package}' has no active export '{export}'",
        optional_message = "; did you mean '{suggestion}'?"
    )]
    MissingPackageExport {
        /// The package specifier.
        anchor: DiagnosticAnchor,
        /// The package name.
        package: String,
        /// The missing export key.
        export: String,
        /// The closest active package export.
        suggestion: Option<String>,
    },

    /// Package export exists but does not point to a source module.
    ///
    /// ```tspp
    /// import asset from "library/style.css";
    /// ```
    #[diagnostic(
        id = "non-module-package-export",
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
    /// ```tspp
    /// import { value } from "library";
    /// ```
    #[diagnostic(
        id = "unloaded-package-dependency",
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
    /// ```tspp
    /// @if
    /// import { debug } from "./debug.tspp";
    /// ```
    #[diagnostic(
        id = "missing-static-import-condition",
        message = "`@if` import guard requires a condition"
    )]
    StaticIfRequiresCondition {
        /// The `@if` decorator.
        anchor: DiagnosticAnchor,
    },

    /// Static import guard has more than one condition argument.
    ///
    /// ```tspp
    /// @if(import.meta.profile == "test", true)
    /// import { debug } from "./debug.tspp";
    /// ```
    #[diagnostic(
        id = "multiple-static-import-conditions",
        message = "`@if` import guard requires exactly one condition"
    )]
    StaticIfRequiresOneArgument {
        /// The `@if` decorator.
        anchor: DiagnosticAnchor,
    },

    /// Static import guard evaluates to a non-boolean value.
    ///
    /// ```tspp
    /// @if(import.meta.profile)
    /// import { debug } from "./debug.tspp";
    /// ```
    #[diagnostic(
        id = "non-boolean-static-import-condition",
        message = "`@if` import guard condition must be boolean"
    )]
    StaticIfRequiresBoolean {
        /// The `@if` condition.
        anchor: DiagnosticAnchor,
    },

    /// Static import guard depends on a value that is not available during import.
    ///
    /// ```tspp
    /// @if(enabled)
    /// import { debug } from "./debug.tspp";
    /// ```
    #[diagnostic(
        id = "non-static-import-condition",
        message = "`@if` import guard condition is not static"
    )]
    StaticIfNotStatic {
        /// The non-static `@if` condition.
        anchor: DiagnosticAnchor,
    },

    /// Static import guard is not invoked in its intrinsic form.
    ///
    /// ```tspp
    /// @if<boolean>(true)
    /// import { debug } from "./debug.tspp";
    /// ```
    #[diagnostic(
        id = "invalid-static-import-condition",
        message = "`@if` import guard must be invoked as `@if(condition)`"
    )]
    InvalidStaticIfInvocation {
        /// The malformed `@if` decorator.
        anchor: DiagnosticAnchor,
    },

    /// Internal import failure.
    #[diagnostic(id = "internal-import-error", message = "internal error: {message}")]
    Internal {
        /// The source that triggered the internal failure.
        anchor: DiagnosticAnchor,
        /// The internal failure message.
        message: String,
    },
}
