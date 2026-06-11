use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Condition, GenericArgument, Origin, TypeOperand, TypeRelation, TypeTerm,
    VariableId,
};

/// Runtime await expression term.
///
/// ```ds
/// await value
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct AwaitTerm {
    /// The source await expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The awaited expression type.
    pub(in crate::check) value: TypeOperand,
}

impl CheckState<'_> {
    /// Reduce one await term.
    pub(in crate::check) fn reduce_await_term(
        &mut self,
        awaited: AwaitTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let Some(term) = self.promise_value_type(awaited.value)? else {
            return Ok(Answer::pending(awaited.value.dependencies(self)));
        };

        Ok(Answer::Ready(term))
    }

    /// Check an awaited operand to produce the expected result.
    pub(in crate::check) fn expect_await_term(
        &mut self,
        origin: Origin,
        awaited: &AwaitTerm,
        result: VariableId,
    ) -> CompilerResult<Answer<()>> {
        let symbol = self.language_symbol(dir::LanguageItem::Promise);
        let argument = GenericArgument::Type(result.into());
        let expected = TypeTerm::Reference {
            origin: Origin::Node(awaited.source),
            symbol,
            arguments: vec![argument].into(),
        };
        let expected = self.inference.push_term(expected);

        self.constrain_type(
            origin,
            TypeRelation::Assignable,
            awaited.value,
            expected,
            Condition::Always,
        );

        Ok(Answer::Ready(()))
    }

    /// Return the fulfilled value type from one promise term.
    fn promise_value_type(&mut self, ty: TypeOperand) -> CompilerResult<Option<TypeOperand>> {
        let Some(term) = self.type_operand_term_id(ty)? else {
            return Ok(None);
        };

        match self.inference.term(term) {
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } if self.is_language_symbol(*symbol, dir::LanguageItem::Promise) => {
                let value = arguments.first().and_then(GenericArgument::type_operand);

                Ok(value)
            }
            _ => Ok(None),
        }
    }
}
