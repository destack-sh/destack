use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    ArgumentTerm, CheckState, ConstraintOrigin, Decision, Progress, TypeLiteralTerm, TypeRelation,
    TypeTerm, VariableId,
};

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

/// Runtime try failure projection.
///
/// ```ts
/// value?
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct TryFailureTerm {
    /// The source try expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The tried expression type.
    pub(in crate::check) value: VariableId,
}

impl TryFailureTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> smallvec::SmallVec<[VariableId; 4]> {
        let mut variables = smallvec::SmallVec::new();
        variables.push(self.value);
        variables
    }
}

/// Runtime yield expression term.
///
/// ```ts
/// yield value
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct YieldTerm {
    /// The source yield expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The yielded value type.
    pub(in crate::check) value: Option<VariableId>,
    /// The current generator yield channel.
    pub(in crate::check) yield_type: Option<VariableId>,
    /// The value received when the generator resumes.
    pub(in crate::check) resume_type: Option<VariableId>,
    /// The completion value of a delegated generator.
    pub(in crate::check) delegate_return_type: Option<VariableId>,
    /// The yield cardinality.
    pub(in crate::check) cardinality: dir::YieldCardinality,
}

impl YieldTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> smallvec::SmallVec<[VariableId; 4]> {
        let mut variables = smallvec::SmallVec::new();

        variables.extend(self.value);
        variables.extend(self.yield_type);
        variables.extend(self.resume_type);
        variables.extend(self.delegate_return_type);

        variables
    }
}

/// Try operator behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TryTermKind {
    /// Propagate failure through the enclosing return type.
    Maybe,
    /// Trap failure and produce the successful value.
    Must,
}

impl CheckState<'_> {
    /// Reduce one await term.
    pub(in crate::check) fn reduce_await_term(
        &mut self,
        awaited: &AwaitTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(term) = self.solved_type_term(awaited.value)? else {
            return Ok(None);
        };

        self.promise_value_type(&term)
    }

    /// Reduce one yield expression result from the active generator channel.
    pub(in crate::check) fn reduce_yield_term(
        &self,
        yielded: &YieldTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let ty = match yielded.cardinality {
            dir::YieldCardinality::Scalar => yielded.resume_type,
            dir::YieldCardinality::Generator => yielded.delegate_return_type,
        };

        Ok(Some(match ty {
            Some(ty) => TypeTerm::Variable(ty),
            None => TypeTerm::Literal(TypeLiteralTerm::Error),
        }))
    }

    /// Expect an awaited operand to produce the expected result.
    pub(in crate::check) fn expect_await_term(
        &mut self,
        awaited: &AwaitTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let symbol = self.language_symbol(awaited.source.module_id, dir::LanguageItem::Promise)?;
        let argument = self.terms.push(ArgumentTerm::Type(result));
        let expected = TypeTerm::Reference {
            source: Some(awaited.source),
            symbol,
            arguments: vec![argument],
        };
        let origin = ConstraintOrigin::Node(awaited.source);
        let expected = self.solve_anonymous_type(result.module, origin, expected)?;

        self.solve_type_assignability(awaited.value, expected)
    }

    /// Reduce one try operator term.
    pub(in crate::check) fn reduce_try_term(
        &mut self,
        module: ModuleId,
        tried: &TryTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let value = self.try_associated_type_variable(module, tried.value, "Value")?;

        Ok(value.map(TypeTerm::Variable))
    }

    /// Reduce one try failure projection.
    pub(in crate::check) fn reduce_try_failure_term(
        &mut self,
        module: ModuleId,
        tried: &TryFailureTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let failure = self.try_associated_type_variable(module, tried.value, "Failure")?;

        Ok(failure.map(TypeTerm::Variable))
    }

    /// Expect a `Try.Value` projection to produce the expected result.
    pub(in crate::check) fn expect_try_term(
        &mut self,
        tried: &TryTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let Some(value) = self.try_associated_type_variable(result.module, tried.value, "Value")?
        else {
            return Ok(Progress::Unchanged);
        };
        let progress = self.solve_type_assignability(value, result)?;

        Ok(progress)
    }

    /// Expect a `Try.Failure` projection to produce the expected result.
    pub(in crate::check) fn expect_try_failure_term(
        &mut self,
        tried: &TryFailureTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let Some(failure) =
            self.try_associated_type_variable(result.module, tried.value, "Failure")?
        else {
            return Ok(Progress::Unchanged);
        };
        let progress = self.solve_type_assignability(failure, result)?;

        Ok(progress)
    }

    /// Expect a yield expression result to match its resume channel.
    pub(in crate::check) fn expect_yield_term(
        &mut self,
        yielded: &YieldTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let source = match yielded.cardinality {
            dir::YieldCardinality::Scalar => yielded.resume_type,
            dir::YieldCardinality::Generator => yielded.delegate_return_type,
        };
        let Some(source) = source else {
            return Ok(Progress::Unchanged);
        };
        let progress = self.solve_type_assignability(source, result)?;

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
                let Some(value) = self.generic_argument_type_variable(arguments, 0) else {
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
        let name = self.input(module).strings.intern(name);
        let key = dir::StaticKey::Name(name);
        let Some(member) = self.member_type_candidate(module, &receiver, &key)? else {
            return Ok(None);
        };
        if !self.member_implements_language_item(member.symbol, dir::LanguageItem::Try)? {
            return Ok(None);
        }

        match member.ty {
            TypeTerm::Variable(variable) => Ok(Some(variable)),
            _ => Ok(None),
        }
    }

    /// Return the `FromFailure<F>` protocol type.
    pub(in crate::check) fn from_failure_type(
        &mut self,
        source: dir::GlobalNodeIdAny,
        module: ModuleId,
        failure: VariableId,
    ) -> CompilerResult<TypeTerm> {
        let symbol = self.language_symbol(module, dir::LanguageItem::FromFailure)?;
        let argument = self.terms.push(ArgumentTerm::Type(failure));

        Ok(TypeTerm::Reference {
            source: Some(source),
            symbol,
            arguments: vec![argument],
        })
    }
}
