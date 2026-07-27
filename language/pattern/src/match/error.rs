use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::ContextError;

/// An invariant violation while matching a compiled pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchError {
    /// A compiled predicate has no checked program context.
    MissingPredicateContext,
    /// Checked DIR context required by a predicate is invalid.
    Context(ContextError),
    /// A compiled predicate expression has no evaluator implementation.
    UnsupportedPredicateExpression,
    /// A predicate metavariable has an incompatible structural binding.
    InvalidPredicateBinding,
    /// A candidate node required for source ordering has no span.
    MissingCandidateSpan,
    /// A candidate name has no source span.
    MissingCandidateNameSpan,
    /// A relational operation has no candidate child index.
    MissingRelationIndex,
    /// A node metavariable has a different binding type.
    IncompatibleNodeBinding,
    /// A name operation received a different metavariable use.
    InvalidNameUse,
    /// A name metavariable has a different binding type.
    IncompatibleNameBinding,
    /// A repeated metavariable has a different binding type.
    IncompatibleNodesBinding,
}

impl Display for MatchError {
    /// Format the violated matching invariant.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::MissingPredicateContext => "compiled predicate has no checked program context",
            Self::Context(error) => return Display::fmt(error, formatter),
            Self::UnsupportedPredicateExpression => {
                "compiled predicate expression has no evaluator"
            }
            Self::InvalidPredicateBinding => {
                "predicate metavariable has an incompatible structural binding"
            }
            Self::MissingCandidateSpan => "candidate node has no source span",
            Self::MissingCandidateNameSpan => "candidate name has no source span",
            Self::MissingRelationIndex => "relational pattern has no candidate child index",
            Self::IncompatibleNodeBinding => "node metavariable has a non-node binding",
            Self::InvalidNameUse => "name binding received a non-name metavariable",
            Self::IncompatibleNameBinding => "name metavariable has a non-name binding",
            Self::IncompatibleNodesBinding => "sequence metavariable has a non-sequence binding",
        };

        formatter.write_str(message)
    }
}

impl Error for MatchError {}

impl From<ContextError> for MatchError {
    /// Convert one checked context failure into a match failure.
    fn from(error: ContextError) -> Self {
        Self::Context(error)
    }
}
