use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{CheckState, GenericArgument, Origin, TypeOperand, TypeTerm, VariableId};

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
        awaited: &AwaitTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(term) = self.type_operand_term(awaited.value)? else {
            return Ok(None);
        };

        self.promise_value_type(&term)
    }

    /// Expect an awaited operand to produce the expected result.
    pub(in crate::check) fn expect_await_term(
        &mut self,
        origin: Origin,
        awaited: &AwaitTerm,
        result: VariableId,
    ) -> CompilerResult<()> {
        let symbol = self.language_symbol(awaited.source.module_id, dir::LanguageItem::Promise);
        let argument = GenericArgument::Type(result.into());
        let expected = TypeTerm::Reference {
            origin: Origin::Node(awaited.source),
            symbol,
            arguments: vec![argument].into(),
        };
        let expected = self.inference.push_term(expected);

        self.reduce_contextual_type_assignability(origin, awaited.value, expected)
    }

    /// Return the fulfilled value type from one promise term.
    fn promise_value_type(&mut self, term: &TypeTerm) -> CompilerResult<Option<TypeTerm>> {
        match term {
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } if self.environment.language.item(*symbol) == Some(dir::LanguageItem::Promise) => {
                let Some(value) = self.type_argument_variable_at(arguments, 0) else {
                    return Ok(None);
                };

                let Some(term) = self.type_operand_term(value.into())? else {
                    return Ok(None);
                };

                Ok(Some(term))
            }
            _ => Ok(None),
        }
    }
}
