use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Condition, FunctionTerm, OperatorDecision, OperatorDispatch,
    OperatorFailure, OperatorFailureReason, OperatorResolution, Origin, TypeLiteralTerm,
    TypeOperand, TypeRelation, TypeTerm, VariableId,
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
        operator: OperatorTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        if let Some(term) = self.selected_operator_type(&operator)? {
            return Ok(Answer::Ready(term));
        }

        let result = self.select_operator(origin, &operator, None)?;
        match &result {
            OperatorDispatch::Builtin { return_type } => {
                self.select_builtin_operator(&operator, *return_type)?;

                let Some(return_type) = self.operator_return_type_operand(*return_type) else {
                    return Ok(Answer::pending(return_type.dependencies(self)));
                };

                return Ok(Answer::Ready(return_type));
            }
            OperatorDispatch::Method {
                symbol,
                function,
                return_type,
            } => {
                self.select_operator_method(&operator, *symbol, &function)?;

                let Some(return_type) = self.operator_return_type_operand(*return_type) else {
                    return Ok(Answer::pending(return_type.dependencies(self)));
                };

                return Ok(Answer::Ready(return_type));
            }
            OperatorDispatch::NoMatch { reason } => {
                self.reject_operator(&operator, *reason)?;

                let term = TypeTerm::Literal(TypeLiteralTerm::Error);
                let operand = self.type_term_operand(term);

                return Ok(Answer::Ready(operand));
            }
            OperatorDispatch::Pending(blockers) => return Ok(Answer::Pending(blockers.clone())),
        }
    }

    /// Expect one runtime operator expression to produce one result type.
    pub(in crate::check) fn expect_operator_term(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        result: VariableId,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Answer<()>> {
        let resolved = self.select_operator(origin, operator, expected)?;

        let answer = match &resolved {
            OperatorDispatch::Builtin { return_type } => {
                self.select_builtin_operator(operator, *return_type)?;

                self.expect_operator_return_type(origin, *return_type, result, expected)?
            }
            OperatorDispatch::Method {
                symbol,
                function,
                return_type,
            } => {
                self.select_operator_method(operator, *symbol, &function)?;

                self.expect_operator_return_type(origin, *return_type, result, expected)?
            }
            OperatorDispatch::NoMatch { reason } => {
                self.reject_operator(operator, *reason)?;

                let return_type = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Error));

                self.expect_operator_return_type(origin, return_type, result, expected)?
            }
            OperatorDispatch::Pending(blockers) => Answer::Pending(blockers.clone()),
        };

        Ok(answer)
    }

    /// Return the type produced by one selected operator decision.
    pub(in crate::check) fn selected_operator_type(
        &mut self,
        operator: &OperatorTerm,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(decision) = self.inference.operator(operator.source) else {
            return Ok(None);
        };

        let term = match decision {
            OperatorDecision::Resolved(OperatorResolution::Builtin { result, .. }) => {
                let Some(term) = self.operator_return_type_operand(*result) else {
                    return Ok(None);
                };

                term
            }
            OperatorDecision::Resolved(OperatorResolution::Method { function, .. }) => {
                let Some(return_type) = function.return_type else {
                    let term = TypeTerm::Literal(TypeLiteralTerm::Void);
                    let operand = self.type_term_operand(term);

                    return Ok(Some(operand));
                };
                let Some(term) = self.operator_return_type_operand(return_type) else {
                    return Ok(None);
                };

                term
            }
            OperatorDecision::Rejected(_) => {
                let term = TypeTerm::Literal(TypeLiteralTerm::Error);

                self.type_term_operand(term)
            }
        };

        Ok(Some(term))
    }

    /// Return one resolved operator return operand.
    fn operator_return_type_operand(&self, return_type: TypeOperand) -> Option<TypeOperand> {
        let return_type = self.resolved_type_operand(return_type)?;

        match return_type {
            TypeOperand::Term(_) | TypeOperand::Type(_) => Some(return_type),
            TypeOperand::Variable(_) => None,
        }
    }

    /// Check one selected operator return type to satisfy the result variable.
    fn expect_operator_return_type(
        &mut self,
        origin: Origin,
        return_type: TypeOperand,
        result: VariableId,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Answer<()>> {
        self.constrain_type(
            origin,
            TypeRelation::Assignable,
            return_type,
            result,
            Condition::Always,
        );

        // push the selected operator result into the contextual expectation
        if let Some(expected) = expected {
            self.constrain_type(
                origin,
                TypeRelation::Assignable,
                return_type,
                expected,
                Condition::Always,
            );
        }

        Ok(Answer::Ready(()))
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

        self.inference.select_operator(operator.source, decision)?;

        Ok(())
    }

    /// Select one resolved builtin operator for commit.
    pub(in crate::check) fn select_builtin_operator(
        &mut self,
        operator: &OperatorTerm,
        result: TypeOperand,
    ) -> CompilerResult<()> {
        let resolution = OperatorResolution::Builtin {
            source: operator.source,
            kind: operator.kind,
            receiver: operator.receiver,
            argument: operator.argument,
            result,
        };

        let decision = OperatorDecision::Resolved(resolution);

        self.inference.select_operator(operator.source, decision)?;

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

        self.inference.select_operator(operator.source, decision)?;

        Ok(())
    }
}
