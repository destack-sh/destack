use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CheckState, FunctionTerm, OperatorCandidateDispatch, OperatorDecision, OperatorFailure,
    OperatorFailureReason, OperatorResolution, Origin, Progress, TypeLiteralTerm, TypeOperand,
    TypeTerm, VariableId,
};

/// Runtime operator expression term.
///
/// ```ds
/// -value
/// left + right
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct OperatorTerm {
    /// The source operator expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The source operator.
    pub(in crate::check) kind: OperatorTermKind,
    /// The receiver operand type.
    pub(in crate::check) receiver: TypeOperand,
    /// The remaining operand type.
    pub(in crate::check) argument: Option<TypeOperand>,
}

impl OperatorTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> smallvec::SmallVec<[VariableId; 2]> {
        let mut variables = smallvec::SmallVec::new();

        variables.extend(self.receiver.referenced_variables(state));
        if let Some(argument) = self.argument {
            variables.extend(argument.referenced_variables(state));
        }

        variables
    }
}

/// Source operator represented by an operator term.
///
/// Examples:
/// ```ds
/// -value
/// left + right
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum OperatorTermKind {
    /// Unary source operator.
    ///
    /// Examples:
    /// ```ds
    /// -value
    /// !flag
    /// ```
    Unary(dir::UnaryOperator),
    /// Binary source operator.
    ///
    /// Examples:
    /// ```ds
    /// left + right
    /// left === right
    /// ```
    Binary(dir::BinaryOperator),
}

impl CheckState<'_> {
    /// Reduce one runtime operator to its result type.
    pub(in crate::check) fn reduce_operator_term(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let result = self.select_operator_candidate(origin, operator, None)?;
        match &result {
            OperatorCandidateDispatch::Builtin { return_type } => {
                return Ok(Some(return_type.clone()));
            }
            OperatorCandidateDispatch::Method {
                symbol,
                function,
                return_type,
            } => {
                self.select_operator_method(operator, *symbol, &function)?;

                return Ok(Some(return_type.clone()));
            }
            OperatorCandidateDispatch::NoMatch { reason } => {
                self.reject_operator(operator, *reason)?;

                return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Error)));
            }
            OperatorCandidateDispatch::Pending => return Ok(None),
        }
    }

    /// Expect resolved operator candidates to produce the expected result.
    pub(in crate::check) fn expect_operator_term(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        result: VariableId,
        expected: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let resolved = self.select_operator_candidate(origin, operator, Some(expected))?;
        let progress = match &resolved {
            OperatorCandidateDispatch::Builtin { return_type } => {
                self.select_builtin_operator(operator, result)?;

                self.expect_operator_return_type(origin, &return_type, expected)?
            }
            OperatorCandidateDispatch::Method {
                symbol,
                function,
                return_type,
            } => {
                self.select_operator_method(operator, *symbol, &function)?;

                self.expect_operator_return_type(origin, &return_type, expected)?
            }
            OperatorCandidateDispatch::NoMatch { reason } => {
                self.reject_operator(operator, *reason)?;

                self.constrain_solved_type_assignable(
                    origin,
                    &TypeTerm::Literal(TypeLiteralTerm::Error),
                    expected,
                )?
            }
            OperatorCandidateDispatch::Pending => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Expect one selected operator return type to satisfy the result variable.
    fn expect_operator_return_type(
        &mut self,
        origin: Origin,
        return_type: &TypeTerm,
        expected: &TypeTerm,
    ) -> CompilerResult<Progress> {
        self.constrain_solved_type_assignable(origin, return_type, expected)
    }

    /// Reject one operator for diagnostics.
    pub(in crate::check) fn reject_operator(
        &mut self,
        operator: &OperatorTerm,
        reason: OperatorFailureReason,
    ) -> CompilerResult<()> {
        let failure = OperatorFailure {
            source: operator.source,
            kind: operator.kind,
            reason,
        };
        let decision = OperatorDecision::Rejected(failure);

        self.select_operator(decision)?;

        Ok(())
    }

    /// Select one resolved builtin operator for commit.
    pub(in crate::check) fn select_builtin_operator(
        &mut self,
        operator: &OperatorTerm,
        result: VariableId,
    ) -> CompilerResult<()> {
        let resolution = OperatorResolution::Builtin {
            source: operator.source,
            kind: operator.kind,
            receiver: operator.receiver,
            argument: operator.argument,
            result,
        };

        let decision = OperatorDecision::Resolved(resolution);

        self.select_operator(decision)?;

        Ok(())
    }

    /// Select one resolved operator method for commit.
    pub(in crate::check) fn select_operator_method(
        &mut self,
        operator: &OperatorTerm,
        symbol: dir::GlobalSymbolId,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        let resolution = OperatorResolution::Method {
            source: operator.source,
            symbol,
            receiver: operator.receiver,
            function: function.clone(),
        };

        let decision = OperatorDecision::Resolved(resolution);

        self.select_operator(decision)?;

        Ok(())
    }
}
