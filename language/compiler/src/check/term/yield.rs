use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{CheckState, Origin, TypeLiteralTerm, TypeOperand, TypeTerm, VariableId};

/// Runtime yield expression term.
///
/// ```ds
/// yield value
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct YieldTerm {
    /// The source yield expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The yielded value type.
    pub(in crate::check) value: Option<TypeOperand>,
    /// The current generator yield target.
    pub(in crate::check) yield_target: Option<TypeOperand>,
    /// The current generator resume target.
    pub(in crate::check) resume_target: Option<TypeOperand>,
    /// The completion target of a delegated generator.
    pub(in crate::check) delegate_return_target: Option<TypeOperand>,
    /// The yield cardinality.
    pub(in crate::check) cardinality: dir::YieldCardinality,
}

impl YieldTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> smallvec::SmallVec<[VariableId; 2]> {
        let mut variables = smallvec::SmallVec::new();

        if let Some(value) = self.value {
            variables.extend(value.referenced_variables(state));
        }
        if let Some(target) = self.yield_target {
            variables.extend(target.referenced_variables(state));
        }
        if let Some(target) = self.resume_target {
            variables.extend(target.referenced_variables(state));
        }
        if let Some(target) = self.delegate_return_target {
            variables.extend(target.referenced_variables(state));
        }

        variables
    }
}

impl CheckState<'_> {
    /// Reduce one yield expression result from the active generator channel.
    pub(in crate::check) fn reduce_yield_term(
        &self,
        yielded: &YieldTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let ty = match yielded.cardinality {
            dir::YieldCardinality::Scalar => yielded.resume_target,
            dir::YieldCardinality::Generator => yielded.delegate_return_target,
        };

        let Some(ty) = ty else {
            return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Error)));
        };
        let Some(term) = self.type_operand_term(ty)? else {
            return Ok(None);
        };

        Ok(Some(term))
    }

    /// Expect a yield expression result to match its resume channel.
    pub(in crate::check) fn expect_yield_term(
        &mut self,
        origin: Origin,
        yielded: &YieldTerm,
        result: VariableId,
    ) -> CompilerResult<()> {
        let source = match yielded.cardinality {
            dir::YieldCardinality::Scalar => yielded.resume_target,
            dir::YieldCardinality::Generator => yielded.delegate_return_target,
        };
        let Some(source) = source else {
            return Ok(());
        };

        self.reduce_contextual_type_assignability(origin, source, result)?;

        Ok(())
    }
}
