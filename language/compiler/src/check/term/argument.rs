use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, GenericSubstitution, Progress, StaticRelation, TermId, TypeRelation,
    VariableId,
};

/// Argument supplied to a generic use.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ArgumentTerm {
    /// Type argument.
    Type(VariableId),
    /// Static argument.
    Static(VariableId),
    /// Argument that can be interpreted as either a type or static value.
    TypeOrStatic {
        /// The type interpretation.
        ty: VariableId,
        /// The static interpretation.
        value: VariableId,
    },
    /// Spread type argument.
    SpreadType(VariableId),
    /// Static spread argument.
    SpreadStatic(VariableId),
    /// Spread argument that can be interpreted as either type or static values.
    SpreadTypeOrStatic {
        /// The type interpretation.
        ty: VariableId,
        /// The static interpretation.
        value: VariableId,
    },
}

impl ArgumentTerm {
    /// Return the variables referenced by this argument.
    pub(in crate::check) fn variables(&self) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Type(variable)
            | Self::Static(variable)
            | Self::SpreadType(variable)
            | Self::SpreadStatic(variable) => {
                variables.push(*variable);
            }
            Self::TypeOrStatic { ty, value } | Self::SpreadTypeOrStatic { ty, value } => {
                variables.push(*ty);
                variables.push(*value);
            }
        }

        variables
    }

    /// Return the type interpretation of this argument.
    pub(in crate::check) fn type_variable(&self) -> Option<VariableId> {
        match self {
            Self::Type(variable) | Self::SpreadType(variable) => Some(*variable),
            Self::TypeOrStatic { ty, .. } | Self::SpreadTypeOrStatic { ty, .. } => Some(*ty),
            Self::Static(_) | Self::SpreadStatic(_) => None,
        }
    }

    /// Return the static interpretation of this argument.
    pub(in crate::check) fn static_variable(&self) -> Option<VariableId> {
        match self {
            Self::Static(variable) | Self::SpreadStatic(variable) => Some(*variable),
            Self::TypeOrStatic { value, .. } | Self::SpreadTypeOrStatic { value, .. } => {
                Some(*value)
            }
            Self::Type(_) | Self::SpreadType(_) => None,
        }
    }

    /// Select this argument for a known generic slot kind.
    pub(in crate::check) fn select_for_static_slot(&self, is_static: bool) -> Self {
        match (self, is_static) {
            (Self::TypeOrStatic { ty, .. }, false) => Self::Type(*ty),
            (Self::TypeOrStatic { value, .. }, true) => Self::Static(*value),
            (Self::SpreadTypeOrStatic { ty, .. }, false) => Self::SpreadType(*ty),
            (Self::SpreadTypeOrStatic { value, .. }, true) => Self::SpreadStatic(*value),
            _ => self.clone(),
        }
    }

    /// Substitute generic arguments through this argument.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let argument = match self {
            Self::Type(variable) => {
                Self::Type(state.substitute_type_variable(module, substitution, *variable)?)
            }
            Self::Static(variable) => {
                Self::Static(state.substitute_static_variable(module, substitution, *variable)?)
            }
            Self::TypeOrStatic { ty, value } => Self::TypeOrStatic {
                ty: state.substitute_type_variable(module, substitution, *ty)?,
                value: state.substitute_static_variable(module, substitution, *value)?,
            },
            Self::SpreadType(variable) => {
                Self::SpreadType(state.substitute_type_variable(module, substitution, *variable)?)
            }
            Self::SpreadStatic(variable) => Self::SpreadStatic(state.substitute_static_variable(
                module,
                substitution,
                *variable,
            )?),
            Self::SpreadTypeOrStatic { ty, value } => Self::SpreadTypeOrStatic {
                ty: state.substitute_type_variable(module, substitution, *ty)?,
                value: state.substitute_static_variable(module, substitution, *value)?,
            },
        };

        Ok(argument)
    }

    /// Substitute generic arguments through an argument list.
    pub(in crate::check) fn substitute_all(
        arguments: &[Self],
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Vec<Self>> {
        arguments
            .iter()
            .map(|argument| argument.substitute(module, substitution, state))
            .collect()
    }
}

impl CheckState<'_> {
    /// Return the variables referenced by one argument term.
    pub(in crate::check) fn argument_variables(
        &self,
        argument: TermId<ArgumentTerm>,
    ) -> SmallVec<[VariableId; 2]> {
        self.terms.get(argument).variables()
    }

    /// Return the type interpretation of one argument term.
    pub(in crate::check) fn argument_type_variable(
        &self,
        argument: TermId<ArgumentTerm>,
    ) -> Option<VariableId> {
        self.terms.get(argument).type_variable()
    }

    /// Return the static interpretation of one argument term.
    pub(in crate::check) fn argument_static_variable(
        &self,
        argument: TermId<ArgumentTerm>,
    ) -> Option<VariableId> {
        self.terms.get(argument).static_variable()
    }

    /// Select one argument term for a known generic slot kind.
    pub(in crate::check) fn select_argument_for_static_slot(
        &mut self,
        argument: TermId<ArgumentTerm>,
        is_static: bool,
    ) -> TermId<ArgumentTerm> {
        let argument = self.terms.get(argument).select_for_static_slot(is_static);

        self.terms.push(argument)
    }

    /// Substitute generic arguments through one argument term.
    pub(in crate::check) fn substitute_argument(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        argument: TermId<ArgumentTerm>,
    ) -> CompilerResult<TermId<ArgumentTerm>> {
        let argument = self.terms.get(argument).substitute(module, substitution, self)?;
        let argument = self.terms.push(argument);

        Ok(argument)
    }

    /// Substitute generic arguments through an argument list.
    pub(in crate::check) fn substitute_arguments(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        arguments: &[TermId<ArgumentTerm>],
    ) -> CompilerResult<Vec<TermId<ArgumentTerm>>> {
        arguments
            .iter()
            .map(|argument| self.substitute_argument(module, substitution, *argument))
            .collect()
    }

    /// Decide exact equality for argument lists.
    pub(in crate::check) fn decide_argument_list_equal(
        &self,
        left: &[TermId<ArgumentTerm>],
        right: &[TermId<ArgumentTerm>],
    ) -> CompilerResult<Decision> {
        if left.len() != right.len() {
            return Ok(Decision::No);
        }

        let mut decision = Decision::Yes;
        for (left, right) in left.iter().zip(right) {
            decision = decision.and(self.decide_argument_equal(*left, *right)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide exact equality for one argument.
    pub(in crate::check) fn decide_argument_equal(
        &self,
        left: TermId<ArgumentTerm>,
        right: TermId<ArgumentTerm>,
    ) -> CompilerResult<Decision> {
        let left = self.terms.get(left);
        let right = self.terms.get(right);
        let decision = match (left, right) {
            (ArgumentTerm::Type(left), ArgumentTerm::Type(right))
            | (ArgumentTerm::SpreadType(left), ArgumentTerm::SpreadType(right)) => {
                self.decide_type_relation(TypeRelation::Equal, *left, *right)?
            }
            (ArgumentTerm::Static(left), ArgumentTerm::Static(right))
            | (ArgumentTerm::SpreadStatic(left), ArgumentTerm::SpreadStatic(right)) => {
                self.decide_static_relation(StaticRelation::Equal, *left, *right)?
            }
            (left, right)
                if let (Some(left), Some(right)) =
                    (left.type_variable(), right.type_variable()) =>
            {
                self.decide_type_relation(TypeRelation::Equal, left, right)?
            }
            (left, right)
                if let (Some(left), Some(right)) =
                    (left.static_variable(), right.static_variable()) =>
            {
                self.decide_static_relation(StaticRelation::Equal, left, right)?
            }
            _ => Decision::No,
        };

        Ok(decision)
    }

    /// Constrain matching generic arguments by exact equality.
    pub(in crate::check) fn constrain_argument_list_equal(
        &mut self,
        left: &[TermId<ArgumentTerm>],
        right: &[TermId<ArgumentTerm>],
    ) -> CompilerResult<Progress> {
        if left.len() != right.len() {
            return Ok(Progress::Unchanged);
        }
        let mut progress = Progress::Unchanged;

        // constrain matching argument slots
        for (left, right) in left.iter().zip(right) {
            progress = progress.merge(self.constrain_argument_equal(*left, *right)?);
        }

        Ok(progress)
    }

    /// Constrain one generic argument by exact equality.
    fn constrain_argument_equal(
        &mut self,
        left: TermId<ArgumentTerm>,
        right: TermId<ArgumentTerm>,
    ) -> CompilerResult<Progress> {
        let left = self.terms.get(left);
        let right = self.terms.get(right);
        let progress = match (left, right) {
            (ArgumentTerm::Type(left), ArgumentTerm::Type(right))
            | (ArgumentTerm::SpreadType(left), ArgumentTerm::SpreadType(right)) => {
                self.solve_type_equality(*left, *right)?
            }
            (ArgumentTerm::Static(left), ArgumentTerm::Static(right))
            | (ArgumentTerm::SpreadStatic(left), ArgumentTerm::SpreadStatic(right)) => {
                self.solve_static_equality(*left, *right)?
            }
            (left, right)
                if let (Some(left), Some(right)) =
                    (left.type_variable(), right.type_variable()) =>
            {
                self.solve_type_equality(left, right)?
            }
            (left, right)
                if let (Some(left), Some(right)) =
                    (left.static_variable(), right.static_variable()) =>
            {
                self.solve_static_equality(left, right)?
            }
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }
}
