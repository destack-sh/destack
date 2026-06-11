use destack_dir as dir;

use crate::check::{
    Answer, CheckState, Condition, Origin, TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm,
    VariableId,
};
use crate::{CompilerError, CompilerResult};

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

impl CheckState<'_> {
    /// Reduce one yield expression result from the active generator channel.
    pub(in crate::check) fn reduce_yield_term(
        &mut self,
        yielded: YieldTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let ty = match yielded.cardinality {
            dir::YieldCardinality::Scalar => yielded.resume_target,
            dir::YieldCardinality::Generator => yielded.delegate_return_target,
        };

        let Some(ty) = ty else {
            let term = TypeTerm::Literal(TypeLiteralTerm::Error);
            let operand = self.type_term_operand(term);

            return Ok(Answer::Ready(operand));
        };
        let Some(term) = self.type_operand_term_id(ty)? else {
            return Ok(Answer::pending(ty.dependencies(self)));
        };

        Ok(Answer::Ready(term.into()))
    }

    /// Check a yield expression result to match its resume channel.
    pub(in crate::check) fn expect_yield_term(
        &mut self,
        origin: Origin,
        yielded: &YieldTerm,
        result: VariableId,
    ) -> CompilerResult<Answer<()>> {
        let source = match yielded.cardinality {
            dir::YieldCardinality::Scalar => yielded.resume_target,
            dir::YieldCardinality::Generator => yielded.delegate_return_target,
        };
        let Some(source) = source else {
            return Err(CompilerError::Internal {
                message: "yield term has no active resume target".into(),
            });
        };

        self.constrain_type(
            origin,
            TypeRelation::Assignable,
            source,
            result,
            Condition::Always,
        );

        Ok(Answer::Ready(()))
    }
}
