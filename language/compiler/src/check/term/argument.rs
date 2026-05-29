use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, GenericSubstitution, Origin, Progress, StaticOperand, StaticRelation,
    TypeOperand, TypeRelation, VariableId,
};

/// Argument supplied to a generic use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum GenericArgument {
    /// Type argument.
    Type(TypeOperand),
    /// Static argument.
    Static(StaticOperand),
    /// Associated type refinement.
    AssociatedType {
        /// The refined associated type name.
        name: dir::StringId,
        /// The refined type value.
        value: TypeOperand,
    },
    /// Associated compile-time constant refinement.
    AssociatedConst {
        /// The refined associated constant name.
        name: dir::StringId,
        /// The refined static value.
        value: StaticOperand,
    },
    /// Argument that can be interpreted as either a type or static value.
    TypeOrStatic {
        /// The type interpretation.
        ty: TypeOperand,
        /// The static interpretation.
        value: StaticOperand,
    },
    /// Spread type argument.
    SpreadType(TypeOperand),
    /// Static spread argument.
    SpreadStatic(StaticOperand),
    /// Spread argument that can be interpreted as either type or static values.
    SpreadTypeOrStatic {
        /// The type interpretation.
        ty: TypeOperand,
        /// The static interpretation.
        value: StaticOperand,
    },
}

impl GenericArgument {
    /// Return the variables referenced by this argument.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Type(operand) | Self::SpreadType(operand) => {
                variables.extend(operand.referenced_variables(state));
            }
            Self::Static(operand)
            | Self::SpreadStatic(operand)
            | Self::AssociatedConst { value: operand, .. } => {
                variables.extend(operand.referenced_variables(state));
            }
            Self::AssociatedType { value: operand, .. } => {
                variables.extend(operand.referenced_variables(state));
            }
            Self::TypeOrStatic { ty, value } | Self::SpreadTypeOrStatic { ty, value } => {
                variables.extend(ty.referenced_variables(state));
                variables.extend(value.referenced_variables(state));
            }
        }

        variables
    }

    /// Return the type interpretation of this argument.
    pub(in crate::check) fn type_operand(&self) -> Option<TypeOperand> {
        match self {
            Self::Type(operand) | Self::SpreadType(operand) => Some(*operand),
            Self::AssociatedType { value, .. } => Some(*value),
            Self::TypeOrStatic { ty, .. } | Self::SpreadTypeOrStatic { ty, .. } => Some(*ty),
            Self::Static(_) | Self::SpreadStatic(_) | Self::AssociatedConst { .. } => None,
        }
    }

    /// Return the static interpretation of this argument.
    pub(in crate::check) fn static_operand(&self) -> Option<StaticOperand> {
        match self {
            Self::Static(operand) | Self::SpreadStatic(operand) => Some(*operand),
            Self::AssociatedConst { value, .. } => Some(*value),
            Self::TypeOrStatic { value, .. } | Self::SpreadTypeOrStatic { value, .. } => {
                Some(*value)
            }
            Self::Type(_) | Self::SpreadType(_) | Self::AssociatedType { .. } => None,
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
            Self::Type(operand) => {
                Self::Type(state.substitute_type_operand(module, substitution, *operand)?)
            }
            Self::Static(operand) => {
                Self::Static(state.substitute_static_operand(module, substitution, *operand)?)
            }
            Self::AssociatedType { name, value } => Self::AssociatedType {
                name: *name,
                value: state.substitute_type_operand(module, substitution, *value)?,
            },
            Self::AssociatedConst { name, value } => Self::AssociatedConst {
                name: *name,
                value: state.substitute_static_operand(module, substitution, *value)?,
            },
            Self::TypeOrStatic { ty, value } => Self::TypeOrStatic {
                ty: state.substitute_type_operand(module, substitution, *ty)?,
                value: state.substitute_static_operand(module, substitution, *value)?,
            },
            Self::SpreadType(operand) => {
                Self::SpreadType(state.substitute_type_operand(module, substitution, *operand)?)
            }
            Self::SpreadStatic(operand) => Self::SpreadStatic(state.substitute_static_operand(
                module,
                substitution,
                *operand,
            )?),
            Self::SpreadTypeOrStatic { ty, value } => Self::SpreadTypeOrStatic {
                ty: state.substitute_type_operand(module, substitution, *ty)?,
                value: state.substitute_static_operand(module, substitution, *value)?,
            },
        };

        Ok(argument)
    }
}

impl CheckState<'_> {
    /// Return the variables referenced by one generic argument.
    pub(in crate::check) fn argument_variables(
        &self,
        argument: &GenericArgument,
    ) -> SmallVec<[VariableId; 2]> {
        argument.referenced_variables(self)
    }

    /// Return the type interpretation of one generic argument.
    pub(in crate::check) fn argument_type_variable(
        &self,
        argument: &GenericArgument,
    ) -> Option<VariableId> {
        argument.type_operand()?.variable()
    }

    /// Return the static interpretation of one generic argument.
    pub(in crate::check) fn argument_static_variable(
        &self,
        argument: &GenericArgument,
    ) -> Option<VariableId> {
        argument.static_operand()?.variable()
    }

    /// Select one generic argument for a known generic slot kind.
    pub(in crate::check) fn select_argument_for_static_slot(
        &self,
        argument: &GenericArgument,
        is_static: bool,
    ) -> GenericArgument {
        argument.select_for_static_slot(is_static)
    }

    /// Substitute generic arguments through one generic argument.
    pub(in crate::check) fn substitute_argument(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        argument: &GenericArgument,
    ) -> CompilerResult<GenericArgument> {
        let argument = argument.substitute(module, substitution, self)?;

        Ok(argument)
    }

    /// Substitute generic arguments through an argument list.
    pub(in crate::check) fn substitute_arguments(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        arguments: &[GenericArgument],
    ) -> CompilerResult<SmallVec<[GenericArgument; 4]>> {
        arguments
            .iter()
            .map(|argument| self.substitute_argument(module, substitution, argument))
            .collect()
    }

    /// Decide exact equality for argument lists.
    pub(in crate::check) fn decide_argument_list_equal(
        &self,
        left: &[GenericArgument],
        right: &[GenericArgument],
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
        left: &GenericArgument,
        right: &GenericArgument,
    ) -> CompilerResult<Decision> {
        let decision = match (left, right) {
            (GenericArgument::Type(left), GenericArgument::Type(right))
            | (GenericArgument::SpreadType(left), GenericArgument::SpreadType(right)) => {
                self.decide_type_relation(TypeRelation::Equal, *left, *right)?
            }
            (GenericArgument::Static(left), GenericArgument::Static(right))
            | (GenericArgument::SpreadStatic(left), GenericArgument::SpreadStatic(right)) => {
                self.decide_static_relation(StaticRelation::Equal, *left, *right)?
            }
            (
                GenericArgument::AssociatedType {
                    name: left_name,
                    value: left,
                },
                GenericArgument::AssociatedType {
                    name: right_name,
                    value: right,
                },
            ) if left_name == right_name => {
                self.decide_type_relation(TypeRelation::Equal, *left, *right)?
            }
            (
                GenericArgument::AssociatedConst {
                    name: left_name,
                    value: left,
                },
                GenericArgument::AssociatedConst {
                    name: right_name,
                    value: right,
                },
            ) if left_name == right_name => {
                self.decide_static_relation(StaticRelation::Equal, *left, *right)?
            }
            (left, right)
                if let (Some(left), Some(right)) = (left.type_operand(), right.type_operand()) =>
            {
                self.decide_type_relation(TypeRelation::Equal, left, right)?
            }
            (left, right)
                if let (Some(left), Some(right)) =
                    (left.static_operand(), right.static_operand()) =>
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
        origin: Origin,
        left: &[GenericArgument],
        right: &[GenericArgument],
    ) -> CompilerResult<Progress> {
        if left.len() != right.len() {
            return Ok(Progress::Unchanged);
        }
        let mut progress = Progress::Unchanged;

        // constrain matching argument slots
        for (left, right) in left.iter().zip(right) {
            progress = progress.merge(self.constrain_argument_equal(origin, left, right)?);
        }

        Ok(progress)
    }

    /// Constrain one generic argument by exact equality.
    fn constrain_argument_equal(
        &mut self,
        origin: Origin,
        left: &GenericArgument,
        right: &GenericArgument,
    ) -> CompilerResult<Progress> {
        let progress = match (left, right) {
            (GenericArgument::Type(left), GenericArgument::Type(right))
            | (GenericArgument::SpreadType(left), GenericArgument::SpreadType(right)) => {
                self.solve_type_equality(origin, *left, *right)?
            }
            (GenericArgument::Static(left), GenericArgument::Static(right))
            | (GenericArgument::SpreadStatic(left), GenericArgument::SpreadStatic(right)) => {
                self.solve_static_equality(*left, *right)?
            }
            (
                GenericArgument::AssociatedType {
                    name: left_name,
                    value: left,
                },
                GenericArgument::AssociatedType {
                    name: right_name,
                    value: right,
                },
            ) if left_name == right_name => self.solve_type_equality(origin, *left, *right)?,
            (
                GenericArgument::AssociatedConst {
                    name: left_name,
                    value: left,
                },
                GenericArgument::AssociatedConst {
                    name: right_name,
                    value: right,
                },
            ) if left_name == right_name => self.solve_static_equality(*left, *right)?,
            (left, right)
                if let (Some(left), Some(right)) = (left.type_operand(), right.type_operand()) =>
            {
                self.solve_type_equality(origin, left, right)?
            }
            (left, right)
                if let (Some(left), Some(right)) =
                    (left.static_operand(), right.static_operand()) =>
            {
                self.solve_static_equality(left, right)?
            }
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }
}
