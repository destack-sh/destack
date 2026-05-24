use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckComponentState, Decision, GenericSubstitution, Progress, TypeRelation, VariableId,
};

/// Shape member term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ShapeMemberTerm {
    /// Shape field.
    Field {
        /// The field key.
        key: dir::StaticKey,
        /// The field type.
        ty: VariableId,
        /// Whether the field is optional.
        is_optional: bool,
        /// Whether the field is readonly.
        is_readonly: bool,
    },
    /// Call signature.
    CallSignature {
        /// The signature type.
        ty: VariableId,
    },
    /// Construct signature.
    ConstructSignature {
        /// The signature type.
        ty: VariableId,
    },
    /// Index signature.
    IndexSignature {
        /// The parameter name.
        name: dir::StringId,
        /// The key type.
        key_type: VariableId,
        /// The value type.
        value_type: VariableId,
        /// Whether the index signature is optional.
        is_optional: bool,
        /// Whether the index signature is readonly.
        is_readonly: bool,
    },
}

impl ShapeMemberTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Field {
                key: _,
                ty,
                is_optional: _,
                is_readonly: _,
            }
            | Self::CallSignature { ty }
            | Self::ConstructSignature { ty } => variables.push(*ty),
            Self::IndexSignature {
                name: _,
                key_type,
                value_type,
                is_optional: _,
                is_readonly: _,
            } => {
                variables.push(*key_type);
                variables.push(*value_type);
            }
        }

        variables
    }

    /// Substitute generic arguments through this shape member.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckComponentState<'_>,
    ) -> CompilerResult<Self> {
        let member = match self {
            Self::Field {
                key,
                ty,
                is_optional,
                is_readonly,
            } => Self::Field {
                key: *key,
                ty: state.substitute_type_variable(module, substitution, *ty)?,
                is_optional: *is_optional,
                is_readonly: *is_readonly,
            },
            Self::CallSignature { ty } => Self::CallSignature {
                ty: state.substitute_type_variable(module, substitution, *ty)?,
            },
            Self::ConstructSignature { ty } => Self::ConstructSignature {
                ty: state.substitute_type_variable(module, substitution, *ty)?,
            },
            Self::IndexSignature {
                name,
                key_type,
                value_type,
                is_optional,
                is_readonly,
            } => Self::IndexSignature {
                name: *name,
                key_type: state.substitute_type_variable(module, substitution, *key_type)?,
                value_type: state.substitute_type_variable(module, substitution, *value_type)?,
                is_optional: *is_optional,
                is_readonly: *is_readonly,
            },
        };

        Ok(member)
    }

    /// Substitute generic arguments through shape members.
    pub(in crate::check) fn substitute_all(
        members: &[Self],
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckComponentState<'_>,
    ) -> CompilerResult<Vec<Self>> {
        members
            .iter()
            .map(|member| member.substitute(module, substitution, state))
            .collect()
    }
}

impl CheckComponentState<'_> {
    /// Decide exact equality for shape members.
    pub(in crate::check) fn decide_shape_members_equal(
        &self,
        left: &[ShapeMemberTerm],
        right: &[ShapeMemberTerm],
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
        &self,
        source: &[ShapeMemberTerm],
        target: &[ShapeMemberTerm],
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

    /// Relate matching shape fields by equality.
    pub(in crate::check) fn relate_shape_members_equal(
        &mut self,
        left: &[ShapeMemberTerm],
        right: &[ShapeMemberTerm],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // propagate common fields in both directions
        for left in left {
            let Some((left_key, left_ty)) = shape_field(left) else {
                continue;
            };
            let Some(right_ty) = shape_field_type(right, left_key) else {
                continue;
            };

            progress = progress.merge(self.relate_type_equal(left_ty, right_ty)?);
        }

        Ok(progress)
    }

    /// Relate matching shape fields by assignability.
    pub(in crate::check) fn relate_shape_members_assignable(
        &mut self,
        source: &[ShapeMemberTerm],
        target: &[ShapeMemberTerm],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // push target field types into source fields
        for target in target {
            let Some((target_key, target_ty)) = shape_field(target) else {
                continue;
            };
            let Some(source_ty) = shape_field_type(source, target_key) else {
                continue;
            };

            progress = progress.merge(self.relate_type_assignable(source_ty, target_ty)?);
        }

        Ok(progress)
    }

    /// Apply expected shape fields to a shape term.
    pub(in crate::check) fn expect_shape_members(
        &mut self,
        members: &[ShapeMemberTerm],
        targets: &[ShapeMemberTerm],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // apply expected types to common fields
        for target in targets {
            let Some((target_key, target_ty)) = shape_field(target) else {
                continue;
            };
            let Some(member_ty) = shape_field_type(members, target_key) else {
                continue;
            };

            progress = progress.merge(self.relate_type_assignable(member_ty, target_ty)?);
        }

        Ok(progress)
    }

    /// Decide exact equality for one shape member.
    fn decide_shape_member_equal(
        &self,
        left: &ShapeMemberTerm,
        right: &ShapeMemberTerm,
    ) -> CompilerResult<Decision> {
        let decision = match (left, right) {
            (
                ShapeMemberTerm::Field {
                    key: left_key,
                    ty: left_type,
                    is_optional: left_optional,
                    is_readonly: left_readonly,
                },
                ShapeMemberTerm::Field {
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
            (
                ShapeMemberTerm::CallSignature { ty: left },
                ShapeMemberTerm::CallSignature { ty: right },
            )
            | (
                ShapeMemberTerm::ConstructSignature { ty: left },
                ShapeMemberTerm::ConstructSignature { ty: right },
            ) => self.decide_type_relation(TypeRelation::Equal, *left, *right)?,
            (
                ShapeMemberTerm::IndexSignature {
                    name: left_name,
                    key_type: left_key,
                    value_type: left_value,
                    is_optional: left_optional,
                    is_readonly: left_readonly,
                },
                ShapeMemberTerm::IndexSignature {
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
        &self,
        source: &[ShapeMemberTerm],
        target: &ShapeMemberTerm,
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

    /// Return a source member matching one target member.
    fn find_shape_member<'a>(
        &self,
        source: &'a [ShapeMemberTerm],
        target: &ShapeMemberTerm,
    ) -> Option<&'a ShapeMemberTerm> {
        source
            .iter()
            .find(|source| Self::shape_member_matches(source, target))
    }

    /// Return whether two shape members have the same lookup key.
    fn shape_member_matches(left: &ShapeMemberTerm, right: &ShapeMemberTerm) -> bool {
        match (left, right) {
            (
                ShapeMemberTerm::Field { key: left, .. },
                ShapeMemberTerm::Field { key: right, .. },
            ) => left == right,
            (ShapeMemberTerm::CallSignature { .. }, ShapeMemberTerm::CallSignature { .. }) => true,
            (
                ShapeMemberTerm::ConstructSignature { .. },
                ShapeMemberTerm::ConstructSignature { .. },
            ) => true,
            (ShapeMemberTerm::IndexSignature { .. }, ShapeMemberTerm::IndexSignature { .. }) => {
                true
            }
            _ => false,
        }
    }

    /// Return whether one shape member can be omitted.
    fn shape_member_is_optional(member: &ShapeMemberTerm) -> bool {
        match member {
            ShapeMemberTerm::Field { is_optional, .. }
            | ShapeMemberTerm::IndexSignature { is_optional, .. } => *is_optional,
            ShapeMemberTerm::CallSignature { .. } | ShapeMemberTerm::ConstructSignature { .. } => {
                false
            }
        }
    }

    /// Decide assignability for two matched shape members.
    fn decide_shape_member_value_assignable(
        &self,
        source: &ShapeMemberTerm,
        target: &ShapeMemberTerm,
    ) -> CompilerResult<Decision> {
        let decision = match (source, target) {
            (
                ShapeMemberTerm::Field {
                    ty: source_type,
                    is_optional: source_optional,
                    is_readonly: source_readonly,
                    ..
                },
                ShapeMemberTerm::Field {
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
                ShapeMemberTerm::CallSignature { ty: source },
                ShapeMemberTerm::CallSignature { ty: target },
            )
            | (
                ShapeMemberTerm::ConstructSignature { ty: source },
                ShapeMemberTerm::ConstructSignature { ty: target },
            ) => self.decide_type_relation(TypeRelation::Assignable, *source, *target)?,
            (
                ShapeMemberTerm::IndexSignature {
                    key_type: source_key,
                    value_type: source_value,
                    is_optional: source_optional,
                    is_readonly: source_readonly,
                    ..
                },
                ShapeMemberTerm::IndexSignature {
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
}

/// Return one shape field key and type.
pub(in crate::check) fn shape_field(
    member: &ShapeMemberTerm,
) -> Option<(dir::StaticKey, VariableId)> {
    match member {
        ShapeMemberTerm::Field {
            key,
            ty,
            is_optional: _,
            is_readonly: _,
        } => Some((key.clone(), *ty)),
        ShapeMemberTerm::CallSignature { ty: _ }
        | ShapeMemberTerm::ConstructSignature { ty: _ }
        | ShapeMemberTerm::IndexSignature {
            name: _,
            key_type: _,
            value_type: _,
            is_optional: _,
            is_readonly: _,
        } => None,
    }
}

/// Return one shape field type by key.
pub(in crate::check) fn shape_field_type(
    members: &[ShapeMemberTerm],
    key: dir::StaticKey,
) -> Option<VariableId> {
    members.iter().find_map(|member| {
        let (member_key, ty) = shape_field(member)?;

        member_key.matches(&key).then_some(ty)
    })
}
