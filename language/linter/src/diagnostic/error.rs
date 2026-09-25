use tspp_artifact::DiagnosticAnchor;
use tspp_artifact_macros::Diagnostic;

/// Errors during the lint phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Linter)]
pub enum LinterError {
    /// Configured lint id is unknown.
    #[diagnostic(
        id = "unknown-configured-lint",
        message = "unknown configured lint '{lint}'"
    )]
    UnknownConfiguredLint {
        /// Report the package containing the linter configuration.
        anchor: DiagnosticAnchor,
        /// The unknown lint id.
        lint: String,
    },

    /// Expected lint did not occur.
    #[diagnostic(
        id = "unmet-lint-expectation",
        message = "expected lint '{lint}' did not occur"
    )]
    UnmetExpectation {
        /// Report the expectation decorator.
        anchor: DiagnosticAnchor,
        /// The expected lint id.
        lint: String,
    },
}
