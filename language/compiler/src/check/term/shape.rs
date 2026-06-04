use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, Origin, Progress, Substitution, TypeOperand, TypeRelation, TypeTerm,
    VariableId,
};

/// Shape member payload.
///
/// Examples:
/// ```ds
/// { name: string }
/// { ...base, name: string }
/// { (value: string): int32 }
/// { new (value: string): User }
/// { [key: string]: int32 }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ShapeMember {
    /// Shape field.
    ///
    /// Examples:
    /// ```ds
    /// { name: string }
    /// ```
    Field {
        /// The field key.
        key: dir::StaticKey,
        /// The field type.
        ty: TypeOperand,
        /// Whether the field is optional.
        is_optional: bool,
        /// Whether the field is readonly.
        is_readonly: bool,
    },
    /// Spread fields.
    ///
    /// Examples:
    /// ```ds
    /// { ...base }
    /// ```
    Spread {
        /// The spread property source.
        origin: Origin,
        /// The spread source type.
        source: TypeOperand,
    },
    /// Call signature.
    ///
    /// Examples:
    /// ```ds
    /// { (value: string): int32 }
    /// ```
    CallSignature {
        /// The signature type.
        ty: TypeOperand,
    },
    /// Construct signature.
    ///
    /// Examples:
    /// ```ds
    /// { new (value: string): User }
    /// ```
    ConstructSignature {
        /// The signature type.
        ty: TypeOperand,
    },
    /// Index signature.
    ///
    /// Examples:
    /// ```ds
    /// { [key: string]: int32 }
    /// ```
    IndexSignature {
        /// The parameter name.
        name: dir::StringId,
        /// The key type.
        key_type: TypeOperand,
        /// The value type.
        value_type: TypeOperand,
        /// Whether the index signature is optional.
        is_optional: bool,
        /// Whether the index signature is readonly.
        is_readonly: bool,
    },
}

/// Structural object shape payload.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ShapeTerm {
    /// The shape members.
    pub(in crate::check) members: SmallVec<[ShapeMember; 2]>,
}

impl ShapeMember {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Field {
                key: _,
                ty,
                is_optional: _,
                is_readonly: _,
            }
            | Self::CallSignature { ty }
            | Self::ConstructSignature { ty } => variables.extend(ty.referenced_variables(state)),
            Self::Spread { origin: _, source } => {
                variables.extend(source.referenced_variables(state))
            }
            Self::IndexSignature {
                name: _,
                key_type,
                value_type,
                is_optional: _,
                is_readonly: _,
            } => {
                variables.extend(key_type.referenced_variables(state));
                variables.extend(value_type.referenced_variables(state));
            }
        }

        variables
    }

    /// Substitute generic arguments through this shape member.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: Substitution<'_>,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let member = match self {
            Self::Field {
                key,
                ty,
                is_optional,
                is_readonly,
            } => Self::Field {
                key: *key,
                ty: state.substitute_type_operand(module, substitution, *ty)?,
                is_optional: *is_optional,
                is_readonly: *is_readonly,
            },
            Self::Spread { origin, source } => Self::Spread {
                origin: *origin,
                source: state.substitute_type_operand(module, substitution, *source)?,
            },
            Self::CallSignature { ty } => Self::CallSignature {
                ty: state.substitute_type_operand(module, substitution, *ty)?,
            },
            Self::ConstructSignature { ty } => Self::ConstructSignature {
                ty: state.substitute_type_operand(module, substitution, *ty)?,
            },
            Self::IndexSignature {
                name,
                key_type,
                value_type,
                is_optional,
                is_readonly,
            } => Self::IndexSignature {
                name: *name,
                key_type: state.substitute_type_operand(module, substitution, *key_type)?,
                value_type: state.substitute_type_operand(module, substitution, *value_type)?,
                is_optional: *is_optional,
                is_readonly: *is_readonly,
            },
        };

        Ok(member)
    }
}

impl CheckState<'_> {
    /// Store one structural shape type.
    pub(in crate::check) fn push_shape_type(
        &mut self,
        members: SmallVec<[ShapeMember; 2]>,
    ) -> TypeTerm {
        let shape = self.inference.push_term(ShapeTerm { members });

        TypeTerm::Shape(shape)
    }

    /// Substitute generic arguments through shape members.
    pub(in crate::check) fn substitute_shape_members(
        &mut self,
        module: ModuleId,
        substitution: Substitution<'_>,
        members: &[ShapeMember],
    ) -> CompilerResult<Vec<ShapeMember>> {
        members
            .iter()
            .map(|member| member.substitute(module, substitution, self))
            .collect()
    }

    /// Decide exact equality for shape members.
    pub(in crate::check) fn decide_shape_members_equal(
        &mut self,
        left: &[ShapeMember],
        right: &[ShapeMember],
    ) -> CompilerResult<Decision> {
        if left.len() != right.len() {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        // compare shape members in source order
        for (left, right) in left.iter().zip(right) {
            decision = decision.and(self.decide_shape_member_equal(left, right)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide structural shape assignability.
    pub(in crate::check) fn decide_shape_assignable(
        &mut self,
        source: &[ShapeMember],
        target: &[ShapeMember],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        // require each target member from the source shape
        for target in target {
            decision = decision.and(self.decide_shape_member_assignable(source, target)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide structural shape constraint satisfaction.
    pub(in crate::check) fn decide_shape_satisfies(
        &mut self,
        source: &[ShapeMember],
        target: &[ShapeMember],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        // require each target member from the source shape
        for target in target {
            decision = decision.and(self.decide_shape_member_satisfies(source, target)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Relate matching shape fields by equality.
    pub(in crate::check) fn constrain_shape_members_equal(
        &mut self,
        origin: Origin,
        left: &[ShapeMember],
        right: &[ShapeMember],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // constrain common fields in both directions
        for left in left {
            let Some((left_key, left_ty)) = shape_field(left) else {
                continue;
            };
            let Some(right_ty) = self.shape_field_type(right, left_key) else {
                continue;
            };

            progress = progress.merge(self.relate_type_equality(origin, left_ty, right_ty)?);
        }

        Ok(progress)
    }

    /// Relate matching shape fields by assignability.
    pub(in crate::check) fn constrain_shape_members_assignable(
        &mut self,
        origin: Origin,
        source: &[ShapeMember],
        target: &[ShapeMember],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // push target field types into source fields
        for target in target {
            let Some((target_key, target_ty)) = shape_field(target) else {
                continue;
            };
            let Some(source_ty) = self.shape_field_type(source, target_key) else {
                continue;
            };

            progress =
                progress.merge(self.relate_type_assignability(origin, source_ty, target_ty)?);
        }

        Ok(progress)
    }

    /// Expect shape fields to satisfy expected fields.
    pub(in crate::check) fn expect_shape_member_terms(
        &mut self,
        origin: Origin,
        members: &[ShapeMember],
        targets: &[ShapeMember],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // expect common fields to satisfy expected types
        for target in targets {
            let Some((target_key, target_ty)) = shape_field(target) else {
                continue;
            };
            let Some(member_ty) = self.shape_field_type(members, target_key) else {
                continue;
            };

            progress = progress
                .merge(self.relate_contextual_type_assignability(origin, member_ty, target_ty)?);
        }

        Ok(progress)
    }

    /// Decide exact equality for one shape member.
    fn decide_shape_member_equal(
        &mut self,
        left: &ShapeMember,
        right: &ShapeMember,
    ) -> CompilerResult<Decision> {
        let decision = match (left, right) {
            (
                ShapeMember::Field {
                    key: left_key,
                    ty: left_type,
                    is_optional: left_optional,
                    is_readonly: left_readonly,
                },
                ShapeMember::Field {
                    key: right_key,
                    ty: right_type,
                    is_optional: right_optional,
                    is_readonly: right_readonly,
                },
            ) => {
                if left_key != right_key
                    || left_optional != right_optional
                    || left_readonly != right_readonly
                {
                    Decision::No
                } else {
                    self.decide_type_relation(TypeRelation::Equal, *left_type, *right_type)?
                }
            }
            (ShapeMember::CallSignature { ty: left }, ShapeMember::CallSignature { ty: right })
            | (
                ShapeMember::ConstructSignature { ty: left },
                ShapeMember::ConstructSignature { ty: right },
            ) => self.decide_type_relation(TypeRelation::Equal, *left, *right)?,
            (
                ShapeMember::IndexSignature {
                    name: left_name,
                    key_type: left_key,
                    value_type: left_value,
                    is_optional: left_optional,
                    is_readonly: left_readonly,
                },
                ShapeMember::IndexSignature {
                    name: right_name,
                    key_type: right_key,
                    value_type: right_value,
                    is_optional: right_optional,
                    is_readonly: right_readonly,
                },
            ) => {
                if left_name != right_name
                    || left_optional != right_optional
                    || left_readonly != right_readonly
                {
                    Decision::No
                } else {
                    let key =
                        self.decide_type_relation(TypeRelation::Equal, *left_key, *right_key)?;
                    let value =
                        self.decide_type_relation(TypeRelation::Equal, *left_value, *right_value)?;

                    key.and(value)
                }
            }
            _ => Decision::No,
        };

        Ok(decision)
    }

    /// Decide assignability for one target shape member.
    fn decide_shape_member_assignable(
        &mut self,
        source: &[ShapeMember],
        target: &ShapeMember,
    ) -> CompilerResult<Decision> {
        let Some(source) = self.find_shape_member(source, target) else {
            return Ok(if Self::shape_member_is_optional(target) {
                Decision::Yes
            } else {
                Decision::No
            });
        };

        self.decide_shape_member_value_assignable(source, target)
    }

    /// Decide constraint satisfaction for one target shape member.
    fn decide_shape_member_satisfies(
        &mut self,
        source: &[ShapeMember],
        target: &ShapeMember,
    ) -> CompilerResult<Decision> {
        let Some(source) = self.find_shape_member(source, target) else {
            return Ok(if Self::shape_member_is_optional(target) {
                Decision::Yes
            } else {
                Decision::No
            });
        };

        self.decide_shape_member_value_satisfies(source, target)
    }

    /// Return a source member matching one target member.
    fn find_shape_member<'a>(
        &self,
        source: &'a [ShapeMember],
        target: &ShapeMember,
    ) -> Option<&'a ShapeMember> {
        source
            .iter()
            .find(|source| Self::shape_member_matches(source, target))
    }

    /// Return whether two shape members have the same lookup key.
    fn shape_member_matches(left: &ShapeMember, right: &ShapeMember) -> bool {
        match (left, right) {
            (ShapeMember::Field { key: left, .. }, ShapeMember::Field { key: right, .. }) => {
                left == right
            }
            (ShapeMember::CallSignature { .. }, ShapeMember::CallSignature { .. }) => true,
            (ShapeMember::ConstructSignature { .. }, ShapeMember::ConstructSignature { .. }) => {
                true
            }
            (ShapeMember::IndexSignature { .. }, ShapeMember::IndexSignature { .. }) => true,
            _ => false,
        }
    }

    /// Return whether one shape member can be omitted.
    fn shape_member_is_optional(member: &ShapeMember) -> bool {
        match member {
            ShapeMember::Field { is_optional, .. }
            | ShapeMember::IndexSignature { is_optional, .. } => *is_optional,
            ShapeMember::Spread {
                origin: _,
                source: _,
            }
            | ShapeMember::CallSignature { .. }
            | ShapeMember::ConstructSignature { .. } => false,
        }
    }

    /// Decide assignability for two matched shape members.
    fn decide_shape_member_value_assignable(
        &mut self,
        source: &ShapeMember,
        target: &ShapeMember,
    ) -> CompilerResult<Decision> {
        let decision = match (source, target) {
            (
                ShapeMember::Field {
                    ty: source_type,
                    is_optional: source_optional,
                    is_readonly: source_readonly,
                    ..
                },
                ShapeMember::Field {
                    ty: target_type,
                    is_optional: target_optional,
                    is_readonly: target_readonly,
                    ..
                },
            ) => {
                if *source_optional && !*target_optional || *source_readonly && !*target_readonly {
                    Decision::No
                } else {
                    self.decide_type_relation(TypeRelation::Assignable, *source_type, *target_type)?
                }
            }
            (
                ShapeMember::CallSignature { ty: source },
                ShapeMember::CallSignature { ty: target },
            )
            | (
                ShapeMember::ConstructSignature { ty: source },
                ShapeMember::ConstructSignature { ty: target },
            ) => self.decide_type_relation(TypeRelation::Assignable, *source, *target)?,
            (
                ShapeMember::IndexSignature {
                    key_type: source_key,
                    value_type: source_value,
                    is_optional: source_optional,
                    is_readonly: source_readonly,
                    ..
                },
                ShapeMember::IndexSignature {
                    key_type: target_key,
                    value_type: target_value,
                    is_optional: target_optional,
                    is_readonly: target_readonly,
                    ..
                },
            ) => {
                if *source_optional && !*target_optional || *source_readonly && !*target_readonly {
                    Decision::No
                } else {
                    let key = self.decide_type_relation(
                        TypeRelation::Assignable,
                        *target_key,
                        *source_key,
                    )?;
                    let value = self.decide_type_relation(
                        TypeRelation::Assignable,
                        *source_value,
                        *target_value,
                    )?;

                    key.and(value)
                }
            }
            _ => Decision::No,
        };

        Ok(decision)
    }

    /// Decide constraint satisfaction for two matched shape members.
    fn decide_shape_member_value_satisfies(
        &mut self,
        source: &ShapeMember,
        target: &ShapeMember,
    ) -> CompilerResult<Decision> {
        let decision = match (source, target) {
            (
                ShapeMember::Field {
                    ty: source_type,
                    is_optional: source_optional,
                    ..
                },
                ShapeMember::Field {
                    ty: target_type,
                    is_optional: target_optional,
                    ..
                },
            ) => {
                if *source_optional && !*target_optional {
                    Decision::No
                } else {
                    self.decide_type_relation(TypeRelation::Satisfies, *source_type, *target_type)?
                }
            }
            (
                ShapeMember::CallSignature { ty: source },
                ShapeMember::CallSignature { ty: target },
            )
            | (
                ShapeMember::ConstructSignature { ty: source },
                ShapeMember::ConstructSignature { ty: target },
            ) => self.decide_type_relation(TypeRelation::Satisfies, *source, *target)?,
            (
                ShapeMember::IndexSignature {
                    key_type: source_key,
                    value_type: source_value,
                    is_optional: source_optional,
                    ..
                },
                ShapeMember::IndexSignature {
                    key_type: target_key,
                    value_type: target_value,
                    is_optional: target_optional,
                    ..
                },
            ) => {
                if *source_optional && !*target_optional {
                    Decision::No
                } else {
                    let key = self.decide_type_relation(
                        TypeRelation::Assignable,
                        *target_key,
                        *source_key,
                    )?;
                    let value = self.decide_type_relation(
                        TypeRelation::Satisfies,
                        *source_value,
                        *target_value,
                    )?;

                    key.and(value)
                }
            }
            _ => Decision::No,
        };

        Ok(decision)
    }

    /// Return one shape field type by key.
    pub(in crate::check) fn shape_field_type(
        &self,
        members: &[ShapeMember],
        key: dir::StaticKey,
    ) -> Option<TypeOperand> {
        members.iter().find_map(|member| {
            let (member_key, ty) = shape_field(member)?;

            member_key.matches(&key).then_some(ty)
        })
    }
}

/// Return one shape field key and type.
pub(in crate::check) fn shape_field(member: &ShapeMember) -> Option<(dir::StaticKey, TypeOperand)> {
    match member {
        ShapeMember::Field {
            key,
            ty,
            is_optional: _,
            is_readonly: _,
        } => Some((key.clone(), *ty)),
        ShapeMember::Spread {
            origin: _,
            source: _,
        } => None,
        ShapeMember::CallSignature { ty: _ }
        | ShapeMember::ConstructSignature { ty: _ }
        | ShapeMember::IndexSignature {
            name: _,
            key_type: _,
            value_type: _,
            is_optional: _,
            is_readonly: _,
        } => None,
    }
}
