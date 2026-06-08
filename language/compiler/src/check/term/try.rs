use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, GenericArgument, Origin, TypeOperand, TypeRelation, TypeTerm, VariableId,
};

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
    ) -> CompilerResult<()> {
        let Some(value) =
            self.try_associated_type_operand(origin, result.module, tried.value, "Value")?
        else {
            return Ok(());
        };

        self.reduce_contextual_type_assignability(origin, value, result)?;

        Ok(())
    }

    /// Expect a try failure projection to produce the expected result.
    pub(in crate::check) fn expect_try_failure_term(
        &mut self,
        origin: Origin,
        tried: &TryFailureTerm,
        result: VariableId,
    ) -> CompilerResult<()> {
        let Some(failure) =
            self.try_associated_type_operand(origin, result.module, tried.value, "Failure")?
        else {
            return Ok(());
        };

        self.reduce_contextual_type_assignability(origin, failure, result)?;

        Ok(())
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
        let target = self.from_residual_type(source, source.module_id, failure)?;
        let Some(return_type) = self.reduce_type_operand(Origin::Node(source), return_type)? else {
            return Ok(Decision::Undecidable);
        };

        let target = self.inference.push_term(target);

        self.decide_type_relation(TypeRelation::Implements, return_type, target)
    }

    /// Return one try associated type.
    fn try_associated_type_operand(
        &mut self,
        _origin: Origin,
        _module: ModuleId,
        _value: TypeOperand,
        _name: &str,
    ) -> CompilerResult<Option<TypeOperand>> {
        Ok(None)
    }

    /// Return the `FromResidual<R>` protocol type.
    pub(in crate::check) fn from_residual_type(
        &mut self,
        source: dir::GlobalNodeIdAny,
        module: ModuleId,
        failure: TypeOperand,
    ) -> CompilerResult<TypeTerm> {
        let symbol = self.language_symbol(module, dir::LanguageItem::FromResidual);
        let argument = GenericArgument::Type(failure);

        Ok(TypeTerm::Reference {
            origin: Origin::Node(source),
            symbol,
            arguments: vec![argument].into(),
        })
    }
}
