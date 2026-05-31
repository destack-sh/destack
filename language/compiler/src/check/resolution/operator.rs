use destack_dir as dir;

use crate::check::{CheckState, FunctionTerm, OperatorTermKind, TypeOperand, VariableId};

/// Solved runtime operator resolved by the solver.
///
/// Examples:
/// ```ds
/// -value
/// left + right
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum OperatorSelection {
    /// Builtin operator behavior.
    ///
    /// Examples:
    /// ```ds
    /// 1 + 2
    /// !flag
    /// ```
    Builtin {
        /// The source operator expression.
        source: dir::GlobalNodeIdAny,
        /// The source operator.
        kind: OperatorTermKind,
        /// The receiver operand type.
        receiver: TypeOperand,
        /// The remaining operand type.
        argument: Option<TypeOperand>,
        /// The result type.
        result: VariableId,
    },
    /// Symbol-backed operator method.
    ///
    /// Examples:
    /// ```ds
    /// left + right
    /// ```
    Method {
        /// The source binary expression.
        source: dir::GlobalNodeIdAny,
        /// The resolved operator method symbol.
        symbol: dir::GlobalSymbolId,
        /// The receiver type.
        receiver: TypeOperand,
        /// The resolved function signature.
        function: FunctionTerm,
    },
}

/// Runtime operator failure resolved by the solver.
///
/// Examples:
/// ```ds
/// left + right
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct OperatorFailure {
    /// The source operator expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The source operator.
    pub(in crate::check) kind: OperatorTermKind,
    /// The reason operator resolution failed.
    pub(in crate::check) reason: OperatorFailureReason,
}

/// Runtime operator failure reason resolved by the solver.
///
/// Examples:
/// ```ds
/// left + right
/// left === right
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum OperatorFailureReason {
    /// No builtin or protocol operator accepted the operands.
    ///
    /// Examples:
    /// ```ds
    /// left + right
    /// ```
    NoMatch,
    /// Strict equality was used with non identity-compatible operands.
    ///
    /// Examples:
    /// ```ds
    /// left === right
    /// ```
    InvalidStrictEquality,
}

/// Runtime operator decision resolved by the solver.
///
/// Examples:
/// ```ds
/// left + right
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum OperatorDecision {
    /// One operator target resolved.
    ///
    /// Examples:
    /// ```ds
    /// left + right
    /// ```
    Resolved(OperatorSelection),
    /// Operator resolution failed.
    ///
    /// Examples:
    /// ```ds
    /// left + right
    /// ```
    Rejected(OperatorFailure),
}

impl CheckState<'_> {
    /// Select one operator decision.
    pub(in crate::check) fn select_operator(&mut self, decision: OperatorDecision) {
        let source = match &decision {
            OperatorDecision::Resolved(operator) => match operator {
                OperatorSelection::Builtin { source, .. }
                | OperatorSelection::Method { source, .. } => *source,
            },
            OperatorDecision::Rejected(failure) => failure.source,
        };

        if let Some(previous) = self.inference.operator(source) {
            assert_eq!(
                previous, decision,
                "check operator {source:?} already has a different decision"
            );

            return;
        }

        self.inference.insert_operator(source, decision);
    }
}
