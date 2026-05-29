use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CheckState, FunctionTerm, OperatorCandidateSelection, OperatorFailure, OperatorFailureReason,
    OperatorResolution, OperatorSelection, Origin, Progress, TypeTerm, VariableId,
};

/// Runtime operator expression term.
///
/// ```ts
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
    pub(in crate::check) receiver: VariableId,
    /// The remaining operand type.
    pub(in crate::check) argument: Option<VariableId>,
}

impl OperatorTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> smallvec::SmallVec<[VariableId; 4]> {
        let mut variables = smallvec::SmallVec::new();

        variables.push(self.receiver);
        variables.extend(self.argument);

        variables
    }
}

/// Source operator represented by an operator term.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum OperatorTermKind {
    /// Unary source operator.
    Unary(dir::UnaryOperator),
    /// Binary source operator.
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
            OperatorCandidateSelection::Builtin { return_type } => {
                return Ok(Some(return_type.clone()));
            }
            OperatorCandidateSelection::Method {
                symbol,
                function,
                return_type,
            } => {
                self.select_operator_method(operator, *symbol, &function)?;

                return Ok(Some(return_type.clone()));
            }
            OperatorCandidateSelection::NoMatch { reason } => {
                self.select_operator_rejection(operator, *reason)?;

                return Ok(None);
            }
            OperatorCandidateSelection::Pending => return Ok(None),
        }
    }

    /// Expect resolved operator candidates to produce the expected result.
    pub(in crate::check) fn expect_operator_term(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let resolved = self.select_operator_candidate(origin, operator, Some(result))?;
        let progress = match &resolved {
            OperatorCandidateSelection::Builtin { return_type } => {
                self.select_builtin_operator(operator, result)?;

                self.expect_operator_return_type(origin, &return_type, result)?
            }
            OperatorCandidateSelection::Method {
                symbol,
                function,
                return_type,
            } => {
                self.select_operator_method(operator, *symbol, &function)?;

                self.expect_operator_return_type(origin, &return_type, result)?
            }
            OperatorCandidateSelection::NoMatch { reason } => {
                self.select_operator_rejection(operator, *reason)?;

                Progress::Unchanged
            }
            OperatorCandidateSelection::Pending => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Expect one selected operator return type to satisfy the result variable.
    fn expect_operator_return_type(
        &mut self,
        origin: Origin,
        return_type: &TypeTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let Some(expected) = self.solved_type_term(result)? else {
            return Ok(Progress::Unchanged);
        };

        self.constrain_solved_type_assignable(origin, return_type, &expected)
    }

    /// Select one rejected operator for diagnostics.
    pub(in crate::check) fn select_operator_rejection(
        &mut self,
        operator: &OperatorTerm,
        reason: OperatorFailureReason,
    ) -> CompilerResult<()> {
        let failure = OperatorFailure {
            source: operator.source,
            kind: operator.kind,
            reason,
        };
        let decision = OperatorSelection::Rejected(failure);

        self.select_operator(decision);

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

        let decision = OperatorSelection::Resolved(resolution);

        self.select_operator(decision);

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

        let decision = OperatorSelection::Resolved(resolution);

        self.select_operator(decision);

        Ok(())
    }
}
