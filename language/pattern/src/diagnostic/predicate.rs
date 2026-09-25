use tspp_artifact::DiagnosticAnchor;
use tspp_artifact_macros::Diagnostic;

/// Errors while compiling a pattern predicate.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Predicate)]
pub enum PredicateError {
    /// Parsed source does not contain exactly one predicate expression.
    #[diagnostic(
        id = "expected-predicate-root",
        message = "expected one predicate expression, found {found}"
    )]
    ExpectedRoot {
        /// Report the complete predicate source.
        anchor: DiagnosticAnchor,
        /// The parsed root count.
        found: usize,
    },

    /// Predicate compilation encountered an internal invariant violation.
    #[diagnostic(id = "internal-predicate-error", message = "internal error: {message}")]
    Internal {
        /// Report the source that exposed the invariant violation.
        anchor: DiagnosticAnchor,
        /// The violated invariant.
        message: String,
    },

    /// A predicate uses repeated marker syntax.
    #[diagnostic(
        id = "repeated-predicate-metavariable",
        message = "predicate metavariables cannot use repeated marker syntax"
    )]
    RepeatedMetavariable {
        /// Report the repeated predicate metavariable.
        anchor: DiagnosticAnchor,
    },

    /// A predicate references a metavariable absent from its pattern.
    #[diagnostic(
        id = "unbound-predicate-metavariable",
        message = "predicate metavariable '{name}' is not bound by the pattern"
    )]
    UnboundMetavariable {
        /// Report the unbound predicate metavariable.
        anchor: DiagnosticAnchor,
        /// The metavariable name without dollar signs.
        name: String,
    },

    /// A predicate metavariable does not occupy an expression.
    #[diagnostic(
        id = "invalid-predicate-metavariable",
        message = "predicate metavariable must occupy an expression"
    )]
    InvalidMetavariable {
        /// Report the predicate metavariable.
        anchor: DiagnosticAnchor,
    },

    /// A predicate expression is outside the supported algebra.
    #[diagnostic(
        id = "unsupported-predicate-expression",
        message = "expression is not part of the pattern predicate language"
    )]
    UnsupportedExpression {
        /// Report the unsupported expression.
        anchor: DiagnosticAnchor,
    },

    /// A predicate operator cannot accept its operands.
    #[diagnostic(
        id = "invalid-predicate-operands",
        message = "operator '{operator}' cannot accept these predicate values"
    )]
    InvalidOperands {
        /// Report the invalid operation.
        anchor: DiagnosticAnchor,
        /// The authored operator.
        operator: String,
    },
}
