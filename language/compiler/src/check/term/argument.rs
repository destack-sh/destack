use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Condition, Dependency, Origin, StaticOperand, StaticRelation,
    SubstitutionSet, TypeOperand, TypeRelation, VariableKind,
};

/// Argument supplied to a generic use.
///
/// Examples:
/// ```ds
/// Box<string>
/// Array<int32, 4>
/// Protocol<Item = string>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// Argument whose parameter kind is not selected yet.
    ///
    /// Examples:
    /// ```ds
    /// value<T>
    /// ```
    TypeOrStatic {
        /// The source syntax.
        source: TypeOrStaticArgument,
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
    /// Spread argument whose parameter kind is not selected yet.
    ///
    /// Examples:
    /// ```ds
    /// Target<...Args>
    /// ```
    SpreadTypeOrStatic {
        /// The source syntax.
        source: TypeOrStaticArgument,
    },
}

/// Source syntax for a generic argument whose parameter kind is not selected yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct TypeOrStaticArgument {
    /// The generic argument node.
    pub(in crate::check) source: dir::GlobalNodeId<dir::GenericArgument>,
    /// The walked type-space operand.
    pub(in crate::check) ty: TypeOperand,
    /// The walked static-space operand.
    pub(in crate::check) r#static: Option<StaticOperand>,
}

impl GenericArgument {
    /// Create one unselected ordinary generic argument.
    pub(in crate::check) fn type_or_static(
        source: dir::GlobalNodeId<dir::GenericArgument>,
        ty: TypeOperand,
        r#static: Option<StaticOperand>,
    ) -> Self {
        Self::TypeOrStatic {
            source: TypeOrStaticArgument {
                source,
                ty,
                r#static,
            },
        }
    }

    /// Create one unselected spread generic argument.
    pub(in crate::check) fn spread_type_or_static(
        source: dir::GlobalNodeId<dir::GenericArgument>,
        ty: TypeOperand,
        r#static: Option<StaticOperand>,
    ) -> Self {
        Self::SpreadTypeOrStatic {
            source: TypeOrStaticArgument {
                source,
                ty,
                r#static,
            },
        }
    }

    /// Return selections referenced by this argument.
    pub(in crate::check) fn dependencies(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[Dependency; 2]> {
        let mut dependencies = SmallVec::new();

        match self {
            Self::Type(operand)
            | Self::SpreadType(operand)
            | Self::AssociatedType { value: operand, .. }
            | Self::TypeOrStatic {
                source: TypeOrStaticArgument { ty: operand, .. },
            }
            | Self::SpreadTypeOrStatic {
                source: TypeOrStaticArgument { ty: operand, .. },
            } => {
                dependencies.extend(operand.dependencies(state));
            }
            Self::Static(operand)
            | Self::SpreadStatic(operand)
            | Self::AssociatedConst { value: operand, .. } => {
                dependencies.extend(operand.dependencies(state));
            }
        }
        if let Self::TypeOrStatic { source } | Self::SpreadTypeOrStatic { source } = self
            && let Some(value) = source.r#static
        {
            dependencies.extend(value.dependencies(state));
        }

        dependencies
    }

    /// Return the type interpretation of this argument.
    pub(in crate::check) fn type_operand(&self) -> Option<TypeOperand> {
        match self {
            Self::Type(operand) | Self::SpreadType(operand) => Some(*operand),
            Self::AssociatedType { value, .. } => Some(*value),
            Self::TypeOrStatic {
                source: TypeOrStaticArgument { ty, .. },
            }
            | Self::SpreadTypeOrStatic {
                source: TypeOrStaticArgument { ty, .. },
            } => Some(*ty),
            Self::Static(_) | Self::SpreadStatic(_) | Self::AssociatedConst { .. } => None,
        }
    }

    /// Return the static interpretation of this argument.
    pub(in crate::check) fn static_operand(&self) -> Option<StaticOperand> {
        match self {
            Self::Static(operand) | Self::SpreadStatic(operand) => Some(*operand),
            Self::AssociatedConst { value, .. } => Some(*value),
            Self::TypeOrStatic { source } | Self::SpreadTypeOrStatic { source } => source.r#static,
            Self::Type(_) | Self::SpreadType(_) | Self::AssociatedType { .. } => None,
        }
    }

    /// Specialize this argument under one known generic parameter kind.
    pub(in crate::check) fn specialize(
        &self,
        kind: VariableKind,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let argument = match (kind, self) {
            (VariableKind::Type, Self::TypeOrStatic { source }) => Self::Type(source.ty),
            (VariableKind::Type, Self::SpreadTypeOrStatic { source }) => {
                Self::SpreadType(source.ty)
            }
            (VariableKind::Static, Self::TypeOrStatic { source }) => {
                let operand = if let Some(operand) = source.r#static {
                    Some(operand)
                } else {
                    state.static_operand_from_type(source.ty)?
                };
                let Some(operand) = operand else {
                    return Ok(Self::TypeOrStatic { source: *source });
                };

                Self::Static(operand)
            }
            (VariableKind::Static, Self::SpreadTypeOrStatic { source }) => {
                let operand = if let Some(operand) = source.r#static {
                    Some(operand)
                } else {
                    state.static_operand_from_type(source.ty)?
                };
                let Some(operand) = operand else {
                    return Ok(Self::SpreadTypeOrStatic { source: *source });
                };

                Self::SpreadStatic(operand)
            }
            (_, argument) => *argument,
        };

        Ok(argument)
    }

    /// Substitute generic arguments through this argument.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
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
            Self::TypeOrStatic { source } => Self::TypeOrStatic {
                source: TypeOrStaticArgument {
                    source: source.source,
                    ty: state.substitute_type_operand(module, substitution, source.ty)?,
                    r#static: source
                        .r#static
                        .map(|operand| {
                            state.substitute_static_operand(module, substitution, operand)
                        })
                        .transpose()?,
                },
            },
            Self::SpreadType(operand) => {
                Self::SpreadType(state.substitute_type_operand(module, substitution, *operand)?)
            }
            Self::SpreadStatic(operand) => Self::SpreadStatic(state.substitute_static_operand(
                module,
                substitution,
                *operand,
            )?),
            Self::SpreadTypeOrStatic { source } => Self::SpreadTypeOrStatic {
                source: TypeOrStaticArgument {
                    source: source.source,
                    ty: state.substitute_type_operand(module, substitution, source.ty)?,
                    r#static: source
                        .r#static
                        .map(|operand| {
                            state.substitute_static_operand(module, substitution, operand)
                        })
                        .transpose()?,
                },
            },
        };

        Ok(argument)
    }

    /// Return whether this argument can be affected by one substitution.
    pub(in crate::check) fn needs_substitution(
        &self,
        substitution: &SubstitutionSet,
        state: &CheckState<'_>,
    ) -> CompilerResult<bool> {
        let needs_substitution = match self {
            Self::Type(operand)
            | Self::SpreadType(operand)
            | Self::AssociatedType { value: operand, .. } => {
                operand.needs_substitution(substitution, state)?
            }
            Self::Static(operand)
            | Self::SpreadStatic(operand)
            | Self::AssociatedConst { value: operand, .. } => {
                operand.needs_substitution(substitution, state)?
            }
            Self::TypeOrStatic { source } | Self::SpreadTypeOrStatic { source } => {
                source.ty.needs_substitution(substitution, state)?
                    || source
                        .r#static
                        .map(|operand| operand.needs_substitution(substitution, state))
                        .transpose()?
                        .unwrap_or(false)
            }
        };

        Ok(needs_substitution)
    }
}

impl CheckState<'_> {
    /// Substitute generic arguments through one generic argument.
    pub(in crate::check) fn substitute_argument(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        argument: &GenericArgument,
    ) -> CompilerResult<GenericArgument> {
        let argument = argument.substitute(module, substitution, self)?;

        Ok(argument)
    }

    /// Substitute generic arguments through an argument list.
    pub(in crate::check) fn substitute_arguments(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        arguments: &[GenericArgument],
    ) -> CompilerResult<SmallVec<[GenericArgument; 2]>> {
        arguments
            .iter()
            .map(|argument| self.substitute_argument(module, substitution, argument))
            .collect()
    }

    /// Decide exact equality for one argument.
    pub(in crate::check) fn decide_argument_equal(
        &mut self,
        left: &GenericArgument,
        right: &GenericArgument,
    ) -> CompilerResult<Answer<bool>> {
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
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Constrain one generic argument by exact equality.
    pub(in crate::check) fn constrain_argument_equal(
        &mut self,
        origin: Origin,
        left: &GenericArgument,
        right: &GenericArgument,
    ) -> CompilerResult<()> {
        match (left, right) {
            (GenericArgument::Type(left), GenericArgument::Type(right))
            | (GenericArgument::SpreadType(left), GenericArgument::SpreadType(right)) => {
                self.constrain_type(
                    origin,
                    TypeRelation::Equal,
                    *left,
                    *right,
                    Condition::Always,
                );
            }
            (GenericArgument::Static(left), GenericArgument::Static(right))
            | (GenericArgument::SpreadStatic(left), GenericArgument::SpreadStatic(right)) => {
                self.constrain_static(
                    origin,
                    StaticRelation::Equal,
                    *left,
                    *right,
                    Condition::Always,
                );
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
                self.constrain_type(
                    origin,
                    TypeRelation::Equal,
                    *left,
                    *right,
                    Condition::Always,
                );
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
                self.constrain_static(
                    origin,
                    StaticRelation::Equal,
                    *left,
                    *right,
                    Condition::Always,
                );
            }
            (left, right)
                if let (Some(left), Some(right)) = (left.type_operand(), right.type_operand()) =>
            {
                self.constrain_type(origin, TypeRelation::Equal, left, right, Condition::Always);
            }
            (left, right)
                if let (Some(left), Some(right)) =
                    (left.static_operand(), right.static_operand()) =>
            {
                self.constrain_static(
                    origin,
                    StaticRelation::Equal,
                    left,
                    right,
                    Condition::Always,
                );
            }
            _ => (),
        }

        Ok(())
    }
}
