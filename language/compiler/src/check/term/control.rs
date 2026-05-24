use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    ArgumentTerm, CheckComponentState, Decision, Progress, TypeRelation, TypeTerm, VariableId,
};
use crate::{CompilerError, CompilerResult};

/// Runtime await expression term.
///
/// ```ts
/// await value
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct AwaitTerm {
    /// The source await expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The awaited expression type.
    pub(in crate::check) value: VariableId,
}

/// Runtime try operator term.
///
/// ```ts
/// result?
/// result!
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct TryTerm {
    /// The source try expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The tried expression type.
    pub(in crate::check) value: VariableId,
    /// The try operator behavior.
    pub(in crate::check) kind: TryTermKind,
}

impl TryTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> smallvec::SmallVec<[VariableId; 4]> {
        let mut variables = smallvec::SmallVec::new();
        variables.push(self.value);
        variables
    }
}

/// Try operator behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TryTermKind {
    /// Propagate failure through the enclosing return type.
    Propagate,
    /// Trap failure and produce the successful value.
    Trap,
}

impl CheckComponentState<'_> {
    /// Reduce one await term.
    pub(in crate::check) fn reduce_await_type(
        &mut self,
        awaited: &AwaitTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(term) = self.solved_type_term(awaited.value)? else {
            return Ok(None);
        };

        self.promise_value_type(&term)
    }

    /// Apply an expected await result to its promise operand.
    pub(in crate::check) fn expect_await_result(
        &mut self,
        awaited: &AwaitTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let Some(symbol) = self.environment.language.symbol(dir::LanguageItem::Promise) else {
            return Err(CompilerError::Internal {
                message: "missing language item: async.Promise".to_owned(),
            });
        };
        let expected = TypeTerm::Reference {
            source: Some(awaited.source),
            symbol,
            arguments: vec![ArgumentTerm::Type(result)],
        };
        let expected = self.push_solved_type_variable(result.module, expected)?;

        self.relate_type_assignable(awaited.value, expected)
    }

    /// Reduce one try operator term.
    pub(in crate::check) fn reduce_try_type(
        &mut self,
        module: ModuleId,
        tried: &TryTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let value = self.try_associated_type_variable(module, tried.value, "Value")?;

        Ok(value.map(TypeTerm::Variable))
    }

    /// Apply an expected try result to the `Try.Value` projection.
    pub(in crate::check) fn expect_try_result(
        &mut self,
        tried: &TryTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let Some(value) = self.try_associated_type_variable(result.module, tried.value, "Value")?
        else {
            return Ok(Progress::Unchanged);
        };
        let progress = self.relate_type_assignable(value, result)?;

        Ok(progress)
    }

    /// Decide whether a propagated try failure fits an enclosing return type.
    pub(in crate::check) fn decide_try_propagation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: VariableId,
        return_type: VariableId,
    ) -> CompilerResult<Decision> {
        let Some(failure) =
            self.try_associated_type_variable(source.module_id, value, "Failure")?
        else {
            return Ok(Decision::Undecidable);
        };
        let target = self.from_failure_type(source, return_type.module, failure)?;
        let Some(return_type) = self.solved_type_term(return_type)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_type_term_relation(TypeRelation::Implements, &return_type, &target)
    }

    /// Return the fulfilled value type from one promise term.
    fn promise_value_type(&mut self, term: &TypeTerm) -> CompilerResult<Option<TypeTerm>> {
        match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(None);
                };

                self.promise_value_type(&term)
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } if self.environment.language.item(*symbol) == Some(dir::LanguageItem::Promise) => {
                let Some(value) = Self::type_argument(arguments, 0) else {
                    return Ok(None);
                };

                Ok(Some(TypeTerm::Variable(value)))
            }
            _ => Ok(None),
        }
    }

    /// Return one `Try` associated type.
    fn try_associated_type_variable(
        &mut self,
        module: ModuleId,
        value: VariableId,
        name: &str,
    ) -> CompilerResult<Option<VariableId>> {
        let Some(receiver) = self.solved_type_term(value)? else {
            return Ok(None);
        };
        let name = self.module(module)?.input.strings.intern(name);
        let key = dir::StaticKey::Name(name);
        let Some(member) = self.member_type_candidate(module, &receiver, &key)? else {
            return Ok(None);
        };
        if !self.member_implements_language_item(member.symbol, dir::LanguageItem::Try)? {
            return Ok(None);
        }

        Ok(Some(member.ty))
    }

    /// Return the `FromFailure<F>` protocol type.
    pub(in crate::check) fn from_failure_type(
        &self,
        source: dir::GlobalNodeIdAny,
        _module: ModuleId,
        failure: VariableId,
    ) -> CompilerResult<TypeTerm> {
        let Some(symbol) = self
            .environment
            .language
            .symbol(dir::LanguageItem::FromFailure)
        else {
            return Err(CompilerError::Internal {
                message: "missing language item: ops.FromFailure".to_owned(),
            });
        };

        Ok(TypeTerm::Reference {
            source: Some(source),
            symbol,
            arguments: vec![ArgumentTerm::Type(failure)],
        })
    }
}
