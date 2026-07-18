use destack_artifact::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Errors during the lint phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Linter)]
pub enum LinterError {
    /// Configured lint selector is unknown.
    #[diagnostic(code = "EL000", message = "unknown lint selector '{selector}'")]
    UnknownConfiguredLint {
        /// Report the package containing the linter configuration.
        anchor: DiagnosticAnchor,
        /// The unknown selector.
        selector: String,
    },

    /// Two configuration entries select the same lint.
    #[diagnostic(
        code = "EL001",
        message = "lint '{lint}' is configured by both '{first}' and '{second}'"
    )]
    DuplicateConfiguredLint {
        /// Report the package containing the linter configuration.
        anchor: DiagnosticAnchor,
        /// The stable lint id.
        lint: String,
        /// The first selector.
        first: String,
        /// The second selector.
        second: String,
    },

    /// Expected lint did not occur.
    #[diagnostic(code = "EL002", message = "expected lint '{lint}' did not occur")]
    UnmetExpectation {
        /// Report the expectation decorator.
        anchor: DiagnosticAnchor,
        /// The expected lint id.
        lint: String,
    },

    /// Diagnostic decorator selector is unknown.
    #[diagnostic(code = "EL003", message = "unknown diagnostic selector '{selector}'")]
    UnknownControlledDiagnostic {
        /// Report the diagnostic selector.
        anchor: DiagnosticAnchor,
        /// The unknown selector.
        selector: String,
    },

    /// Diagnostic decorator overrides an enclosing forbid.
    #[diagnostic(
        code = "EL004",
        message = "diagnostic '{selector}' is forbidden in this scope"
    )]
    ForbiddenDiagnosticOverride {
        /// Report the rejected diagnostic selector.
        anchor: DiagnosticAnchor,
        /// The rejected selector.
        selector: String,
    },
}
