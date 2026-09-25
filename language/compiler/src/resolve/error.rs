use crate::DiagnosticAnchor;
use tspp_artifact_macros::Diagnostic;
use tspp_source::ModuleId;

/// Errors during the resolve phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Resolve)]
pub enum ResolveError {
    /// Import or re-export selects a name that is not exported by the target module.
    ///
    /// ```tspp
    /// import { missing } from "./library.tspp";
    /// export { missing } from "./library.tspp";
    /// ```
    #[diagnostic(
        id = "missing-export",
        message = "missing export '{name}' from '{target}'",
        optional_message = "; did you mean '{suggestion}'?"
    )]
    MissingExport {
        /// The import or re-export clause.
        anchor: DiagnosticAnchor,
        /// The missing export key.
        name: String,
        /// The resolved target module.
        target: String,
        /// The closest exported key.
        suggestion: Option<String>,
    },

    /// Import or re-export selects a name that exists in the target module without being exported.
    ///
    /// ```tspp
    /// import { hidden } from "./library.tspp";
    /// ```
    #[diagnostic(
        id = "not-exported",
        message = "'{name}' exists in '{target}' but is not exported"
    )]
    NotExported {
        /// The import or re-export clause.
        anchor: DiagnosticAnchor,
        /// The unexported binding name.
        name: String,
        /// The resolved target module.
        target: String,
    },

    /// Import or re-export selects a name that is re-exported by multiple star exports.
    ///
    /// ```tspp
    /// export * from "./left.tspp";
    /// export * from "./right.tspp";
    /// import { shared } from "./barrel.tspp";
    /// ```
    #[diagnostic(
        id = "ambiguous-export",
        message = "ambiguous export '{name}' from '{target}'"
    )]
    AmbiguousExport {
        /// The import or re-export clause.
        anchor: DiagnosticAnchor,
        /// The ambiguous export key.
        name: String,
        /// The resolved target module.
        target: String,
    },

    /// Internal resolve failure.
    #[diagnostic(
        id = "internal-resolution-error",
        message = "internal error: {message}"
    )]
    Internal {
        /// The source that triggered the internal failure.
        anchor: DiagnosticAnchor,
        /// The module being resolved.
        module: ModuleId,
        /// The internal failure message.
        message: String,
    },
}
