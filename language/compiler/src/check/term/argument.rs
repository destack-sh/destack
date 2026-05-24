use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckComponentState, Decision, GenericSubstitution, StaticRelation, TypeRelation, VariableId,
};

/// Argument supplied to a generic use.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ArgumentTerm {
    /// Type argument.
    Type(VariableId),
    /// Static argument.
    Static(VariableId),
    /// Spread type argument.
    SpreadType(VariableId),
    /// Static spread argument.
    SpreadStatic(VariableId),
}

impl ArgumentTerm {
    /// Return the variable referenced by this argument.
    pub(in crate::check) fn variable(&self) -> VariableId {
        match self {
            Self::Type(variable)
            | Self::Static(variable)
            | Self::SpreadType(variable)
            | Self::SpreadStatic(variable) => *variable,
        }
    }

    /// Substitute generic arguments through this argument.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckComponentState<'_>,
    ) -> CompilerResult<Self> {
        let argument = match self {
            Self::Type(variable) => {
                Self::Type(state.substitute_type_variable(module, substitution, *variable)?)
            }
            Self::Static(variable) => {
                Self::Static(state.substitute_static_variable(module, substitution, *variable)?)
            }
            Self::SpreadType(variable) => {
                Self::SpreadType(state.substitute_type_variable(module, substitution, *variable)?)
            }
            Self::SpreadStatic(variable) => Self::SpreadStatic(state.substitute_static_variable(
                module,
                substitution,
                *variable,
            )?),
        };

        Ok(argument)
    }

    /// Substitute generic arguments through an argument list.
    pub(in crate::check) fn substitute_all(
        arguments: &[Self],
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckComponentState<'_>,
    ) -> CompilerResult<Vec<Self>> {
        arguments
            .iter()
            .map(|argument| argument.substitute(module, substitution, state))
            .collect()
    }
}

impl CheckComponentState<'_> {
    /// Decide exact equality for argument lists.
    pub(in crate::check) fn decide_argument_list_equal(
        &self,
        left: &[ArgumentTerm],
        right: &[ArgumentTerm],
    ) -> CompilerResult<Decision> {
        if left.len() != right.len() {
            return Ok(Decision::No);
        }

        let mut decision = Decision::Yes;
        for (left, right) in left.iter().zip(right) {
            decision = decision.and(self.decide_argument_equal(left, right)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide exact equality for one argument.
    pub(in crate::check) fn decide_argument_equal(
        &self,
        left: &ArgumentTerm,
        right: &ArgumentTerm,
    ) -> CompilerResult<Decision> {
        let decision = match (left, right) {
            (ArgumentTerm::Type(left), ArgumentTerm::Type(right))
            | (ArgumentTerm::SpreadType(left), ArgumentTerm::SpreadType(right)) => {
                self.decide_type_relation(TypeRelation::Equal, *left, *right)?
            }
            (ArgumentTerm::Static(left), ArgumentTerm::Static(right))
            | (ArgumentTerm::SpreadStatic(left), ArgumentTerm::SpreadStatic(right)) => {
                self.decide_static_relation(StaticRelation::Equal, *left, *right)?
            }
            _ => Decision::No,
        };

        Ok(decision)
    }
}
