use tspp_artifact::DiagnosticAnchor;
use tspp_artifact_macros::Diagnostic;

/// Errors while compiling a structural pattern.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Pattern)]
pub enum PatternError {
    /// Parsed source does not contain exactly one pattern root.
    #[diagnostic(
        id = "expected-pattern-root",
        message = "expected one pattern root, found {found}"
    )]
    ExpectedRoot {
        /// Report the complete pattern source.
        anchor: DiagnosticAnchor,
        /// The parsed root count.
        found: usize,
    },

    /// A context does not contain exactly one node of the selected type.
    #[diagnostic(
        id = "expected-pattern-context-node",
        message = "expected one {node_type} in pattern context, found {found}"
    )]
    ExpectedContextNode {
        /// Report the complete pattern context.
        anchor: DiagnosticAnchor,
        /// The selected DIR node type.
        node_type: String,
        /// The matching node count.
        found: usize,
    },

    /// Pattern compilation encountered an internal invariant violation.
    #[diagnostic(id = "internal-pattern-error", message = "internal error: {message}")]
    Internal {
        /// Report the source that exposed the invariant violation.
        anchor: DiagnosticAnchor,
        /// The violated invariant.
        message: String,
    },

    /// A metavariable does not occupy a capturable DIR position.
    #[diagnostic(
        id = "invalid-pattern-metavariable",
        message = "metavariable does not occupy a capturable DIR position"
    )]
    InvalidMetavariable {
        /// Report the metavariable.
        anchor: DiagnosticAnchor,
    },

    /// A repeated metavariable does not occupy a repeated DIR list.
    #[diagnostic(
        id = "invalid-repeated-metavariable",
        message = "repeated metavariable does not occupy a repeated DIR list"
    )]
    InvalidRepeatedMetavariable {
        /// Report the repeated metavariable.
        anchor: DiagnosticAnchor,
    },

    /// Uses of one metavariable require incompatible value types.
    #[diagnostic(
        id = "incompatible-pattern-metavariable",
        message = "metavariable '{name}' has incompatible uses"
    )]
    IncompatibleMetavariable {
        /// Report the incompatible use.
        anchor: DiagnosticAnchor,
        /// The metavariable name without dollar signs.
        name: String,
    },

    /// Adjacent repeated metavariables have no unique candidate partition.
    #[diagnostic(
        id = "ambiguous-repeated-metavariables",
        message = "adjacent repeated metavariables have no unique partition"
    )]
    AmbiguousRepeatedMetavariables {
        /// Report the second repeated metavariable.
        anchor: DiagnosticAnchor,
    },
}
