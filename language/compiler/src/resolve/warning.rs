use tspp_artifact_macros::Diagnostic;

use crate::DiagnosticAnchor;

/// Warnings during the resolve phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Resolve)]
pub enum ResolveWarning {
    /// One file imports the same resolved module more than once.
    ///
    /// ```tspp
    /// import { left } from "./values.tspp";
    /// import { right } from "./values.tspp";
    /// ```
    #[diagnostic(
        id = "duplicate-import",
        message = "module '{specifier}' is imported more than once"
    )]
    DuplicateImport {
        /// Report the repeated import declaration.
        anchor: DiagnosticAnchor,
        /// The repeated module specifier.
        specifier: String,
    },
}
