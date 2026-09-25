use tspp_artifact::DiagnosticAnchor;
use tspp_artifact_macros::Diagnostic;
use tspp_source::{
    Diagnostic, DiagnosticCollection, DiagnosticLabel, DiagnosticTarget, File, Span,
};

/// Errors while compiling or selecting structural rewrites.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Rewrite)]
pub enum RewriteError {
    /// Parsed source does not contain exactly one replacement root.
    #[diagnostic(
        id = "expected-replacement-root",
        message = "expected one replacement root, found {found}"
    )]
    ExpectedRoot {
        /// Report the complete replacement source.
        anchor: DiagnosticAnchor,
        /// The parsed root count.
        found: usize,
    },

    /// A context does not contain exactly one replacement node of the selected type.
    #[diagnostic(
        id = "expected-replacement-context-node",
        message = "expected one {node_type} in replacement context, found {found}"
    )]
    ExpectedContextNode {
        /// Report the complete replacement context.
        anchor: DiagnosticAnchor,
        /// The selected DIR node type.
        node_type: String,
        /// The matching node count.
        found: usize,
    },

    /// A replacement metavariable does not occupy a substitutable DIR position.
    #[diagnostic(
        id = "invalid-replacement-metavariable",
        message = "replacement metavariable does not occupy a substitutable DIR position"
    )]
    InvalidMetavariable {
        /// Report the replacement metavariable.
        anchor: DiagnosticAnchor,
    },

    /// A repeated replacement metavariable does not occupy a repeated DIR list.
    #[diagnostic(
        id = "invalid-repeated-replacement-metavariable",
        message = "repeated replacement metavariable does not occupy a repeated DIR list"
    )]
    InvalidRepeatedMetavariable {
        /// Report the repeated replacement metavariable.
        anchor: DiagnosticAnchor,
    },

    /// Adjacent repeated replacement metavariables have no unique partition.
    #[diagnostic(
        id = "ambiguous-repeated-replacement-metavariables",
        message = "adjacent repeated replacement metavariables have no unique partition"
    )]
    AmbiguousRepeatedMetavariables {
        /// Report the second repeated replacement metavariable.
        anchor: DiagnosticAnchor,
    },

    /// An anonymous metavariable appears in a replacement.
    #[diagnostic(
        id = "anonymous-replacement-metavariable",
        message = "anonymous metavariables cannot be used in replacements"
    )]
    AnonymousMetavariable {
        /// Report the anonymous metavariable.
        anchor: DiagnosticAnchor,
    },

    /// A replacement references a metavariable absent from its pattern.
    #[diagnostic(
        id = "unbound-replacement-metavariable",
        message = "replacement metavariable '{name}' is not bound by the pattern"
    )]
    UnboundMetavariable {
        /// Report the unbound replacement metavariable.
        anchor: DiagnosticAnchor,
        /// The metavariable name without dollar signs.
        name: String,
    },

    /// A replacement use requires a different value type from its pattern declaration.
    #[diagnostic(
        id = "incompatible-replacement-metavariable",
        message = "replacement metavariable '{name}' has an incompatible use"
    )]
    IncompatibleMetavariable {
        /// Report the incompatible replacement use.
        anchor: DiagnosticAnchor,
        /// The metavariable name without dollar signs.
        name: String,
    },

    /// Rewrite execution encountered an internal invariant violation.
    #[diagnostic(id = "internal-rewrite-error", message = "internal error: {message}")]
    Internal {
        /// Report the source that exposed the invariant violation.
        anchor: DiagnosticAnchor,
        /// The violated invariant.
        message: String,
    },

    /// Two selected rewrites overlap in candidate source.
    #[diagnostic(id = "overlapping-rewrites", message = "selected rewrites overlap")]
    Overlapping {
        /// Report the later overlapping rewrite.
        anchor: DiagnosticAnchor,
    },
}

impl RewriteError {
    /// Report one rewrite execution invariant violation.
    pub(crate) fn internal(file: &File, error: impl std::fmt::Display) -> DiagnosticCollection {
        let message = error.to_string();
        let error = Self::Internal {
            anchor: file.id.into(),
            message: message.clone(),
        };
        let primary = DiagnosticLabel::new(file.blob(), DiagnosticTarget::File(file.id));
        let diagnostic =
            Diagnostic::error(error.id(), format!("internal error: {message}"), primary);

        DiagnosticCollection::from_diagnostics(vec![diagnostic])
    }

    /// Report two overlapping selected rewrites.
    pub(crate) fn overlapping(
        file: &File,
        selected: Span,
        overlapping: Span,
    ) -> DiagnosticCollection {
        let error = Self::Overlapping {
            anchor: overlapping.into(),
        };
        let blob = file.blob();
        let primary = DiagnosticLabel::message(
            blob,
            DiagnosticTarget::Span(overlapping),
            "this rewrite overlaps another selected rewrite",
        );
        let selected = DiagnosticLabel::message(
            blob,
            DiagnosticTarget::Span(selected),
            "the other rewrite was selected here",
        );
        let diagnostic = Diagnostic::error(error.id(), "selected rewrites overlap", primary)
            .label(selected)
            .help("make the pattern select non-overlapping roots");

        DiagnosticCollection::from_diagnostics(vec![diagnostic])
    }
}
