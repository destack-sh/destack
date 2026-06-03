use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, GenericArgument, Origin, Progress, TypeLiteralTerm, TypeOperand,
    TypeRelation, TypeTerm, VariableId,
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

/// Runtime try operator term.
///
/// ```ds
/// result?
/// result!
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct TryTerm {
    /// The source try expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The tried expression type.
    pub(in crate::check) value: TypeOperand,
    /// The try operator behavior.
    pub(in crate::check) kind: TryTermKind,
}

impl TryTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> smallvec::SmallVec<[VariableId; 2]> {
        let mut variables = smallvec::SmallVec::new();
        variables.extend(self.value.referenced_variables(state));
        variables
    }
}

/// Runtime try failure projection.
///
/// ```ds
/// value?
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct TryFailureTerm {
    /// The source try expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The tried expression type.
    pub(in crate::check) value: TypeOperand,
}

impl TryFailureTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> smallvec::SmallVec<[VariableId; 2]> {
        let mut variables = smallvec::SmallVec::new();
        variables.extend(self.value.referenced_variables(state));
        variables
    }
}

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
    pub(in crate::check) yield_target: Option<VariableId>,
    /// The current generator resume target.
    pub(in crate::check) resume_target: Option<VariableId>,
    /// The completion target of a delegated generator.
    pub(in crate::check) delegate_return_target: Option<VariableId>,
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
        variables.extend(self.yield_target);
        variables.extend(self.resume_target);
        variables.extend(self.delegate_return_target);

        variables
    }
}

/// Try operator behavior.
///
/// Examples:
/// ```ds
/// result?
/// result!
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TryTermKind {
    /// Propagate failure through the enclosing return type.
    ///
    /// Examples:
    /// ```ds
    /// result?
    /// ```
    Maybe,
    /// Trap failure and produce the successful value.
    ///
    /// Examples:
    /// ```ds
    /// result!
    /// ```
    Must,
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
        let Some(term) = self.type_operand_term(ty.into())? else {
            return Ok(None);
        };

        Ok(Some(term))
    }

    /// Expect an awaited operand to produce the expected result.
    pub(in crate::check) fn expect_await_term(
        &mut self,
        origin: Origin,
        awaited: &AwaitTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let symbol = self.language_symbol(awaited.source.module_id, dir::LanguageItem::Promise);
        let argument = GenericArgument::Type(result.into());
        let expected = TypeTerm::Reference {
            origin: Origin::Node(awaited.source),
            symbol,
            arguments: vec![argument].into(),
        };
        let expected = self.inference.push_term(expected);

        self.relate_contextual_type_assignability(origin, awaited.value, expected)
    }

    /// Reduce one try operator term.
    pub(in crate::check) fn reduce_try_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        tried: &TryTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(value) = self.try_associated_type_operand(origin, module, tried.value, "Value")?
        else {
            return Ok(None);
        };
        let Some(value) = self.type_operand_term(value)? else {
            return Ok(None);
        };

        Ok(Some(value))
    }

    /// Reduce one try failure projection.
    pub(in crate::check) fn reduce_try_failure_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        tried: &TryFailureTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(failure) =
            self.try_associated_type_operand(origin, module, tried.value, "Failure")?
        else {
            return Ok(None);
        };
        let Some(failure) = self.type_operand_term(failure)? else {
            return Ok(None);
        };

        Ok(Some(failure))
    }

    /// Expect a try value projection to produce the expected result.
    pub(in crate::check) fn expect_try_term(
        &mut self,
        origin: Origin,
        tried: &TryTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let Some(value) =
            self.try_associated_type_operand(origin, result.module, tried.value, "Value")?
        else {
            return Ok(Progress::Unchanged);
        };
        let progress = self.relate_contextual_type_assignability(origin, value, result)?;

        Ok(progress)
    }

    /// Expect a try failure projection to produce the expected result.
    pub(in crate::check) fn expect_try_failure_term(
        &mut self,
        origin: Origin,
        tried: &TryFailureTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let Some(failure) =
            self.try_associated_type_operand(origin, result.module, tried.value, "Failure")?
        else {
            return Ok(Progress::Unchanged);
        };
        let progress = self.relate_contextual_type_assignability(origin, failure, result)?;

        Ok(progress)
    }

    /// Expect a yield expression result to match its resume channel.
    pub(in crate::check) fn expect_yield_term(
        &mut self,
        origin: Origin,
        yielded: &YieldTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let source = match yielded.cardinality {
            dir::YieldCardinality::Scalar => yielded.resume_target,
            dir::YieldCardinality::Generator => yielded.delegate_return_target,
        };
        let Some(source) = source else {
            return Ok(Progress::Unchanged);
        };
        let progress = self.relate_contextual_type_assignability(origin, source, result)?;

        Ok(progress)
    }

    /// Decide whether a propagated try failure fits an enclosing return type.
    pub(in crate::check) fn reduce_try_propagation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: TypeOperand,
        return_type: TypeOperand,
    ) -> CompilerResult<Decision> {
        let Some(failure) = self.try_associated_type_operand(
            Origin::Node(source),
            source.module_id,
            value,
            "Failure",
        )?
        else {
            return Ok(Decision::Undecidable);
        };
        let target = self.from_failure_type(source, source.module_id, failure)?;
        let Some(return_type) = self.reduce_type_operand(Origin::Node(source), return_type)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_type_term_relation(TypeRelation::Implements, &return_type, &target)
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

    /// Return one try associated type.
    fn try_associated_type_operand(
        &mut self,
        _origin: Origin,
        _module: ModuleId,
        _value: TypeOperand,
        _name: &str,
    ) -> CompilerResult<Option<TypeOperand>> {
        todo!("resolve try associated types through nominal protocol table")
    }

    /// Return the `FromFailure<F>` protocol type.
    pub(in crate::check) fn from_failure_type(
        &mut self,
        source: dir::GlobalNodeIdAny,
        module: ModuleId,
        failure: TypeOperand,
    ) -> CompilerResult<TypeTerm> {
        let symbol = self.language_symbol(module, dir::LanguageItem::FromFailure);
        let argument = GenericArgument::Type(failure);

        Ok(TypeTerm::Reference {
            origin: Origin::Node(source),
            symbol,
            arguments: vec![argument].into(),
        })
    }
}
