use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, Origin, Progress, StaticOperand, StaticRelation, Substitution,
    TypeOperand, TypeRelation, VariableId,
};

/// Argument supplied to a generic use.
///
/// Examples:
/// ```ds
/// Box<string>
/// Array<int32, 4>
/// Protocol<Item = string>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum GenericArgument {
    /// Type argument.
    ///
    /// Examples:
    /// ```ds
    /// Box<string>
    /// ```
    Type(TypeOperand),
    /// Static argument.
    ///
    /// Examples:
    /// ```ds
    /// Array<int32, 4>
    /// ```
    Static(StaticOperand),
    /// Associated type refinement.
    ///
    /// Examples:
    /// ```ds
    /// Iterator<Item = string>
    /// ```
    AssociatedType {
        /// The refined associated type name.
        name: dir::StringId,
        /// The refined type value.
        value: TypeOperand,
    },
    /// Associated compile-time constant refinement.
    ///
    /// Examples:
    /// ```ds
    /// Buffer<Capacity = 4>
    /// ```
    AssociatedConst {
        /// The refined associated constant name.
        name: dir::StringId,
        /// The refined static value.
        value: StaticOperand,
    },
    /// Argument whose slot kind is not selected yet.
    ///
    /// Examples:
    /// ```ds
    /// value<T>
    /// ```
    TypeOrStatic {
        /// The candidate interpretations.
        value: Box<TypeOrStaticOperand>,
    },
    /// Spread type argument.
    ///
    /// Examples:
    /// ```ds
    /// Tuple<...Items>
    /// ```
    SpreadType(TypeOperand),
    /// Static spread argument.
    ///
    /// Examples:
    /// ```ds
    /// Array<...Lengths>
    /// ```
    SpreadStatic(StaticOperand),
    /// Spread argument whose slot kind is not selected yet.
    ///
    /// Examples:
    /// ```ds
    /// Target<...Args>
    /// ```
    SpreadTypeOrStatic {
        /// The candidate interpretations.
        value: Box<TypeOrStaticOperand>,
    },
}

/// Candidate interpretations for an unselected generic argument.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TypeOrStaticOperand {
    /// The type interpretation.
    pub(in crate::check) ty: TypeOperand,
    /// The static interpretation.
    pub(in crate::check) value: StaticOperand,
}

impl GenericArgument {
    /// Create one unselected ordinary generic argument.
    pub(in crate::check) fn type_or_static(ty: TypeOperand, value: StaticOperand) -> Self {
        Self::TypeOrStatic {
            value: Box::new(TypeOrStaticOperand { ty, value }),
        }
    }

    /// Create one unselected spread generic argument.
    pub(in crate::check) fn spread_type_or_static(ty: TypeOperand, value: StaticOperand) -> Self {
        Self::SpreadTypeOrStatic {
            value: Box::new(TypeOrStaticOperand { ty, value }),
        }
    }

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
            Self::TypeOrStatic { value } | Self::SpreadTypeOrStatic { value } => {
                variables.extend(value.ty.referenced_variables(state));
                variables.extend(value.value.referenced_variables(state));
            }
        }

        variables
    }

    /// Return the type interpretation of this argument.
    pub(in crate::check) fn type_operand(&self) -> Option<TypeOperand> {
        match self {
            Self::Type(operand) | Self::SpreadType(operand) => Some(*operand),
            Self::AssociatedType { value, .. } => Some(*value),
            Self::TypeOrStatic { value } | Self::SpreadTypeOrStatic { value } => Some(value.ty),
            Self::Static(_) | Self::SpreadStatic(_) | Self::AssociatedConst { .. } => None,
        }
    }

    /// Return the static interpretation of this argument.
    pub(in crate::check) fn static_operand(&self) -> Option<StaticOperand> {
        match self {
            Self::Static(operand) | Self::SpreadStatic(operand) => Some(*operand),
            Self::AssociatedConst { value, .. } => Some(*value),
            Self::TypeOrStatic { value } | Self::SpreadTypeOrStatic { value } => Some(value.value),
            Self::Type(_) | Self::SpreadType(_) | Self::AssociatedType { .. } => None,
        }
    }

    /// Select this argument for a known generic parameter kind.
    pub(in crate::check) fn select_for_static_parameter(&self, is_static: bool) -> Self {
        match (self, is_static) {
            (Self::TypeOrStatic { value }, false) => Self::Type(value.ty),
            (Self::TypeOrStatic { value }, true) => Self::Static(value.value),
            (Self::SpreadTypeOrStatic { value }, false) => Self::SpreadType(value.ty),
            (Self::SpreadTypeOrStatic { value }, true) => Self::SpreadStatic(value.value),
            _ => self.clone(),
        }
    }

    /// Substitute generic arguments through this argument.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: Substitution<'_>,
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
            Self::TypeOrStatic { value } => Self::type_or_static(
                state.substitute_type_operand(module, substitution, value.ty)?,
                state.substitute_static_operand(module, substitution, value.value)?,
            ),
            Self::SpreadType(operand) => {
                Self::SpreadType(state.substitute_type_operand(module, substitution, *operand)?)
            }
            Self::SpreadStatic(operand) => Self::SpreadStatic(state.substitute_static_operand(
                module,
                substitution,
                *operand,
            )?),
            Self::SpreadTypeOrStatic { value } => Self::spread_type_or_static(
                state.substitute_type_operand(module, substitution, value.ty)?,
                state.substitute_static_operand(module, substitution, value.value)?,
            ),
        };

        Ok(argument)
    }
}

impl CheckState<'_> {
    /// Substitute generic arguments through one generic argument.
    pub(in crate::check) fn substitute_argument(
        &mut self,
        module: ModuleId,
        substitution: Substitution<'_>,
        argument: &GenericArgument,
    ) -> CompilerResult<GenericArgument> {
        let argument = argument.substitute(module, substitution, self)?;

        Ok(argument)
    }

    /// Substitute generic arguments through an argument list.
    pub(in crate::check) fn substitute_arguments(
        &mut self,
        module: ModuleId,
        substitution: Substitution<'_>,
        arguments: &[GenericArgument],
    ) -> CompilerResult<SmallVec<[GenericArgument; 2]>> {
        arguments
            .iter()
            .map(|argument| self.substitute_argument(module, substitution, argument))
            .collect()
    }

    /// Decide exact equality for argument lists.
    pub(in crate::check) fn decide_argument_list_equal(
        &mut self,
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
        &mut self,
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
                self.relate_type_equality(origin, *left, *right)?
            }
            (GenericArgument::Static(left), GenericArgument::Static(right))
            | (GenericArgument::SpreadStatic(left), GenericArgument::SpreadStatic(right)) => {
                self.relate_static_equality(*left, *right)?
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
            ) if left_name == right_name => self.relate_type_equality(origin, *left, *right)?,
            (
                GenericArgument::AssociatedConst {
                    name: left_name,
                    value: left,
                },
                GenericArgument::AssociatedConst {
                    name: right_name,
                    value: right,
                },
            ) if left_name == right_name => self.relate_static_equality(*left, *right)?,
            (left, right)
                if let (Some(left), Some(right)) = (left.type_operand(), right.type_operand()) =>
            {
                self.relate_type_equality(origin, left, right)?
            }
            (left, right)
                if let (Some(left), Some(right)) =
                    (left.static_operand(), right.static_operand()) =>
            {
                self.relate_static_equality(left, right)?
            }
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }
}
