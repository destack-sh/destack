use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Condition, Definition, GenericArgument, Origin, SubstitutionSet, TermId,
    TypeOperand, TypeRelation, TypeTerm,
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// Substitute generic arguments through this shape member.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
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
        substitution: &SubstitutionSet,
        members: &[ShapeMember],
    ) -> CompilerResult<Vec<ShapeMember>> {
        members
            .iter()
            .map(|member| member.substitute(module, substitution, self))
            .collect()
    }

    /// Decide exact equality for two shape terms.
    pub(in crate::check) fn decide_shape_terms_equal(
        &mut self,
        left: TermId<ShapeTerm>,
        right: TermId<ShapeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let left_len = self.inference.term(left).members.len();
        let right_len = self.inference.term(right).members.len();
        if left_len != right_len {
            return Ok(Answer::Ready(false));
        }
        let mut decision = Answer::Ready(true);

        // compare shape members in source order
        for index in 0..left_len {
            let left = self.inference.term(left).members[index];
            let right = self.inference.term(right).members[index];

            decision = decision.and(self.decide_shape_member_equal(&left, &right)?);
            if decision == Answer::Ready(false) {
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
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);

        // require each target member from the source shape
        for target in target {
            decision = decision.and(self.decide_shape_member_assignable(source, target)?);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide structural assignability for two shape terms.
    pub(in crate::check) fn decide_shape_terms_assignable(
        &mut self,
        source: TermId<ShapeTerm>,
        target: TermId<ShapeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let target_len = self.inference.term(target).members.len();
        let mut decision = Answer::Ready(true);

        // require each target member from the source shape
        for index in 0..target_len {
            let target = self.inference.term(target).members[index];

            decision = decision.and(self.decide_shape_term_member_assignable(source, &target)?);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide structural satisfaction for two shape terms.
    pub(in crate::check) fn decide_shape_terms_satisfies(
        &mut self,
        source: TermId<ShapeTerm>,
        target: TermId<ShapeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let target_len = self.inference.term(target).members.len();
        let mut decision = Answer::Ready(true);

        // require each target member from the source shape
        for index in 0..target_len {
            let target = self.inference.term(target).members[index];

            decision = decision.and(self.decide_shape_term_member_satisfies(source, &target)?);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Relate matching fields from two shape terms by equality.
    pub(in crate::check) fn constrain_shape_terms_equal(
        &mut self,
        origin: Origin,
        left: TermId<ShapeTerm>,
        right: TermId<ShapeTerm>,
    ) -> CompilerResult<()> {
        let left_len = self.inference.term(left).members.len();

        // constrain common fields in both directions
        for index in 0..left_len {
            let left = self.inference.term(left).members[index];
            let Some((left_key, left_ty)) = shape_field(&left) else {
                continue;
            };
            let Some(right_ty) = self.shape_term_field_type(right, left_key) else {
                continue;
            };

            self.constrain_type(
                origin,
                TypeRelation::Equal,
                left_ty,
                right_ty,
                Condition::Always,
            );
        }

        Ok(())
    }

    /// Relate matching shape fields by assignability.
    pub(in crate::check) fn constrain_shape_members_assignable(
        &mut self,
        origin: Origin,
        source: &[ShapeMember],
        target: &[ShapeMember],
    ) -> CompilerResult<()> {
        // push target field types into source fields
        for target in target {
            let Some((target_key, target_ty)) = shape_field(target) else {
                continue;
            };
            let Some(source_ty) = self.shape_field_type(source, target_key) else {
                continue;
            };

            self.constrain_type(
                origin,
                TypeRelation::Assignable,
                source_ty,
                target_ty,
                Condition::Always,
            );
        }

        Ok(())
    }

    /// Relate matching fields from two shape terms by assignability.
    pub(in crate::check) fn constrain_shape_terms_assignable(
        &mut self,
        origin: Origin,
        source: TermId<ShapeTerm>,
        target: TermId<ShapeTerm>,
    ) -> CompilerResult<()> {
        let target_len = self.inference.term(target).members.len();

        // push target field types into source fields
        for index in 0..target_len {
            let target = self.inference.term(target).members[index];
            let Some((target_key, target_ty)) = shape_field(&target) else {
                continue;
            };
            let Some(source_ty) = self.shape_term_field_type(source, target_key) else {
                continue;
            };

            self.constrain_type(
                origin,
                TypeRelation::Assignable,
                source_ty,
                target_ty,
                Condition::Always,
            );
        }

        Ok(())
    }

    /// Decide assignability from a structural shape to an interface.
    pub(in crate::check) fn decide_shape_assignable_to_interface(
        &mut self,
        source: &[ShapeMember],
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Answer<bool>> {
        let Some(target) = self.interface_shape_members(symbol, arguments)? else {
            return Ok(Answer::Ready(false));
        };

        self.decide_shape_assignable(source, &target)
    }

    /// Decide assignability from a declaration reference to a structural shape.
    pub(in crate::check) fn decide_reference_assignable_to_shape(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        target: &[ShapeMember],
    ) -> CompilerResult<Answer<bool>> {
        let Some(source) = self.reference_shape_members(symbol, arguments)? else {
            return Ok(Answer::Ready(false));
        };

        self.decide_shape_assignable(&source, target)
    }

    /// Constrain a structural shape to an interface.
    pub(in crate::check) fn constrain_shape_assignable_to_interface(
        &mut self,
        origin: Origin,
        source: &[ShapeMember],
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<()> {
        let Some(target) = self.interface_shape_members(symbol, arguments)? else {
            return Ok(());
        };

        self.constrain_shape_members_assignable(origin, source, &target)
    }

    /// Constrain a declaration reference to a structural shape.
    pub(in crate::check) fn constrain_reference_assignable_to_shape(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        target: &[ShapeMember],
    ) -> CompilerResult<()> {
        let Some(source) = self.reference_shape_members(symbol, arguments)? else {
            return Ok(());
        };

        self.constrain_shape_members_assignable(origin, &source, target)
    }

    /// Decide exact equality for one shape member.
    fn decide_shape_member_equal(
        &mut self,
        left: &ShapeMember,
        right: &ShapeMember,
    ) -> CompilerResult<Answer<bool>> {
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
                    Answer::Ready(false)
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
                    Answer::Ready(false)
                } else {
                    let key =
                        self.decide_type_relation(TypeRelation::Equal, *left_key, *right_key)?;
                    let value =
                        self.decide_type_relation(TypeRelation::Equal, *left_value, *right_value)?;

                    key.and(value)
                }
            }
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide assignability for one target shape member.
    fn decide_shape_member_assignable(
        &mut self,
        source: &[ShapeMember],
        target: &ShapeMember,
    ) -> CompilerResult<Answer<bool>> {
        let Some(source) = self.matching_shape_member(source, target) else {
            return Ok(if Self::shape_member_is_optional(target) {
                Answer::Ready(true)
            } else {
                Answer::Ready(false)
            });
        };

        self.decide_shape_member_value_assignable(source, target)
    }

    /// Decide assignability for one target member against a source shape term.
    fn decide_shape_term_member_assignable(
        &mut self,
        source: TermId<ShapeTerm>,
        target: &ShapeMember,
    ) -> CompilerResult<Answer<bool>> {
        let Some(source) = self.matching_shape_term_member(source, target) else {
            return Ok(if Self::shape_member_is_optional(target) {
                Answer::Ready(true)
            } else {
                Answer::Ready(false)
            });
        };

        self.decide_shape_member_value_assignable(&source, target)
    }

    /// Decide satisfaction for one target member against a source shape term.
    fn decide_shape_term_member_satisfies(
        &mut self,
        source: TermId<ShapeTerm>,
        target: &ShapeMember,
    ) -> CompilerResult<Answer<bool>> {
        let Some(source) = self.matching_shape_term_member(source, target) else {
            return Ok(if Self::shape_member_is_optional(target) {
                Answer::Ready(true)
            } else {
                Answer::Ready(false)
            });
        };

        self.decide_shape_member_value_satisfies(&source, target)
    }

    /// Return a source member matching one target member.
    fn matching_shape_member<'a>(
        &self,
        source: &'a [ShapeMember],
        target: &ShapeMember,
    ) -> Option<&'a ShapeMember> {
        source
            .iter()
            .find(|source| Self::shape_member_matches(source, target))
    }

    /// Return a source member matching one target member.
    fn matching_shape_term_member(
        &self,
        source: TermId<ShapeTerm>,
        target: &ShapeMember,
    ) -> Option<ShapeMember> {
        self.inference
            .term(source)
            .members
            .iter()
            .copied()
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
    ) -> CompilerResult<Answer<bool>> {
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
                    Answer::Ready(false)
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
                    Answer::Ready(false)
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
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide constraint satisfaction for two matched shape members.
    fn decide_shape_member_value_satisfies(
        &mut self,
        source: &ShapeMember,
        target: &ShapeMember,
    ) -> CompilerResult<Answer<bool>> {
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
                    Answer::Ready(false)
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
                    Answer::Ready(false)
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
            _ => Answer::Ready(false),
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

    /// Return one shape term field type by key.
    fn shape_term_field_type(
        &self,
        shape: TermId<ShapeTerm>,
        key: dir::StaticKey,
    ) -> Option<TypeOperand> {
        self.inference
            .term(shape)
            .members
            .iter()
            .find_map(|member| {
                let (member_key, ty) = shape_field(member)?;

                member_key.matches(&key).then_some(ty)
            })
    }

    /// Return the structural members declared by one interface reference.
    fn interface_shape_members(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Option<SmallVec<[ShapeMember; 2]>>> {
        let fields = {
            let Some(definition) = self.definitions.definition(symbol) else {
                return Ok(None);
            };
            let Definition::Interface(definition) = definition else {
                return Ok(None);
            };
            if definition.is_nominal {
                return Ok(None);
            }

            definition
                .fields
                .iter()
                .map(|field| (field.key, field.ty))
                .collect::<Vec<_>>()
        };
        let substitution = self.generic_substitution(symbol, arguments)?;
        let mut members = SmallVec::with_capacity(fields.len());

        // substitute interface fields into the applied reference
        for (key, ty) in fields {
            let ty = if substitution.is_empty() {
                ty
            } else {
                self.substitute_type_operand(symbol.module_id, &substitution, ty)?
            };

            members.push(ShapeMember::Field {
                key,
                ty,
                is_optional: false,
                is_readonly: false,
            });
        }

        Ok(Some(members))
    }

    /// Return structural members available through one declaration reference.
    fn reference_shape_members(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Option<SmallVec<[ShapeMember; 2]>>> {
        let members = {
            let Some(definition) = self.definitions.definition(symbol) else {
                return Ok(None);
            };

            definition
                .named_type_members()
                .into_iter()
                .filter_map(|member| Some((member.key(), member.value()?)))
                .collect::<Vec<_>>()
        };
        let substitution = self.generic_substitution(symbol, arguments)?;
        let mut shape = SmallVec::with_capacity(members.len());

        // substitute declaration members into the applied reference
        for (key, ty) in members {
            let ty = if substitution.is_empty() {
                ty
            } else {
                self.substitute_type_operand(symbol.module_id, &substitution, ty)?
            };

            shape.push(ShapeMember::Field {
                key,
                ty,
                is_optional: false,
                is_readonly: false,
            });
        }

        Ok(Some(shape))
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
