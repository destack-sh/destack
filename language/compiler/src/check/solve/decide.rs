use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    ArgumentTerm, CheckComponentState, FormTerm, FunctionTerm, ShapeMemberTerm, StaticRelation,
    StaticTerm, TupleElementTerm, TypeRelation, TypeTerm, VariableId,
};

/// A relation decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum Decision {
    /// The relation is true.
    Yes,
    /// The relation is false.
    No,
    /// The relation cannot be decided from current information.
    Undecidable,
}

impl Decision {
    /// Combine decisions that must both hold.
    pub(in crate::check) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::No, _) | (_, Self::No) => Self::No,
            (Self::Undecidable, _) | (_, Self::Undecidable) => Self::Undecidable,
            (Self::Yes, Self::Yes) => Self::Yes,
        }
    }
}

impl CheckComponentState<'_> {
    /// Decide one type term relation.
    pub(in crate::check) fn decide_type_term_relation(
        &self,
        relation: TypeRelation,
        left: &TypeTerm,
        right: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let decision = match relation {
            TypeRelation::Equal => self.decide_type_equal(left, right)?,
            TypeRelation::Assignable => self.decide_type_assignable(left, right)?,
            TypeRelation::Castable => self.decide_type_castable(left, right)?,
            TypeRelation::Satisfies | TypeRelation::Extends | TypeRelation::Implements => {
                self.decide_type_assignable(left, right)?
            }
        };

        Ok(decision)
    }

    /// Decide one solved type relation.
    pub(in crate::check) fn decide_type_relation(
        &self,
        relation: TypeRelation,
        left: VariableId,
        right: VariableId,
    ) -> CompilerResult<Decision> {
        let Some(left) = self.solved_type_term(left)? else {
            return Ok(Decision::Undecidable);
        };
        let Some(right) = self.solved_type_term(right)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_type_term_relation(relation, &left, &right)
    }

    /// Decide one static term relation.
    pub(in crate::check) fn decide_static_term_relation(
        &self,
        relation: StaticRelation,
        left: &StaticTerm,
        right: &StaticTerm,
    ) -> CompilerResult<Decision> {
        let decision = match relation {
            StaticRelation::Equal => self.decide_static_equal(left, right)?,
        };

        Ok(decision)
    }

    /// Decide one solved static relation.
    pub(in crate::check) fn decide_static_relation(
        &self,
        relation: StaticRelation,
        left: VariableId,
        right: VariableId,
    ) -> CompilerResult<Decision> {
        let Some(left) = self.solved_static_term(left)? else {
            return Ok(Decision::Undecidable);
        };
        let Some(right) = self.solved_static_term(right)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_static_term_relation(relation, &left, &right)
    }

    /// Decide whether an explicit cast is valid.
    fn decide_type_castable(
        &self,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let forward = self.decide_type_assignable(source, target)?;
        if forward == Decision::Yes {
            return Ok(Decision::Yes);
        }

        let backward = self.decide_type_assignable(target, source)?;
        if backward == Decision::Yes {
            return Ok(Decision::Yes);
        }
        if forward == Decision::Undecidable || backward == Decision::Undecidable {
            return Ok(Decision::Undecidable);
        }

        Ok(Decision::No)
    }

    /// Decide exact type equality.
    fn decide_type_equal(&self, left: &TypeTerm, right: &TypeTerm) -> CompilerResult<Decision> {
        if left == right {
            return Ok(Decision::Yes);
        }

        let decision = match (left, right) {
            (TypeTerm::Variable(left), right) => {
                let Some(left) = self.solved_type_term(*left)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_type_equal(&left, right)?
            }
            (left, TypeTerm::Variable(right)) => {
                let Some(right) = self.solved_type_term(*right)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_type_equal(left, &right)?
            }
            (
                TypeTerm::Form {
                    form: left_form,
                    payload: left_value,
                },
                TypeTerm::Form {
                    form: right_form,
                    payload: right_value,
                },
            ) => {
                let form = self.decide_form_equal(left_form, right_form)?;
                if form != Decision::Yes {
                    return Ok(form);
                }

                self.decide_type_relation(TypeRelation::Equal, *left_value, *right_value)?
            }
            (TypeTerm::Literal(left), TypeTerm::Literal(right)) => {
                self.decide_dir_type_equal(left, right)
            }
            (
                TypeTerm::Reference {
                    source: _,
                    symbol: left_symbol,
                    arguments: left_arguments,
                },
                TypeTerm::Reference {
                    source: _,
                    symbol: right_symbol,
                    arguments: right_arguments,
                },
            ) => {
                if left_symbol != right_symbol {
                    Decision::No
                } else {
                    self.decide_argument_list_equal(left_arguments, right_arguments)?
                }
            }
            (TypeTerm::Array { element: left }, TypeTerm::Array { element: right }) => {
                self.decide_type_relation(TypeRelation::Equal, *left, *right)?
            }
            (
                TypeTerm::Slice {
                    element: left_element,
                    is_readonly: left_readonly,
                },
                TypeTerm::Slice {
                    element: right_element,
                    is_readonly: right_readonly,
                },
            ) => {
                if left_readonly != right_readonly {
                    Decision::No
                } else {
                    self.decide_type_relation(TypeRelation::Equal, *left_element, *right_element)?
                }
            }
            (
                TypeTerm::FixedArray {
                    element: left_element,
                    length: left_length,
                    is_readonly: left_readonly,
                },
                TypeTerm::FixedArray {
                    element: right_element,
                    length: right_length,
                    is_readonly: right_readonly,
                },
            ) => {
                if left_readonly != right_readonly {
                    Decision::No
                } else {
                    let element = self.decide_type_relation(
                        TypeRelation::Equal,
                        *left_element,
                        *right_element,
                    )?;
                    let length = self.decide_static_relation(
                        StaticRelation::Equal,
                        *left_length,
                        *right_length,
                    )?;

                    element.and(length)
                }
            }
            (
                TypeTerm::Tuple {
                    form: left_form,
                    elements: left_elements,
                    is_readonly: left_readonly,
                },
                TypeTerm::Tuple {
                    form: right_form,
                    elements: right_elements,
                    is_readonly: right_readonly,
                },
            ) => {
                if left_form != right_form || left_readonly != right_readonly {
                    Decision::No
                } else {
                    self.decide_tuple_elements_equal(left_elements, right_elements)?
                }
            }
            (TypeTerm::Shape { members: left }, TypeTerm::Shape { members: right }) => {
                self.decide_shape_members_equal(left, right)?
            }
            (TypeTerm::Function(left), TypeTerm::Function(right)) => {
                self.decide_function_equal(left, right)?
            }
            (
                TypeTerm::Range {
                    start: left_start,
                    end: left_end,
                    is_inclusive: left_inclusive,
                },
                TypeTerm::Range {
                    start: right_start,
                    end: right_end,
                    is_inclusive: right_inclusive,
                },
            ) => {
                if left_start == right_start
                    && left_end == right_end
                    && left_inclusive == right_inclusive
                {
                    Decision::Yes
                } else {
                    Decision::No
                }
            }
            (TypeTerm::Union { elements: left }, TypeTerm::Union { elements: right })
            | (
                TypeTerm::Intersection { elements: left },
                TypeTerm::Intersection { elements: right },
            ) => self.decide_type_variable_list_equal(left, right)?,
            _ => Decision::No,
        };

        Ok(decision)
    }

    /// Decide assignability from source to target.
    fn decide_type_assignable(
        &self,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        if self.decide_type_equal(source, target)? == Decision::Yes {
            return Ok(Decision::Yes);
        }

        let decision = match (source, target) {
            (TypeTerm::Variable(source), target) => {
                let Some(source) = self.solved_type_term(*source)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_type_assignable(&source, target)?
            }
            (source, TypeTerm::Variable(target)) => {
                let Some(target) = self.solved_type_term(*target)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_type_assignable(source, &target)?
            }
            (
                TypeTerm::Form {
                    form: source_form,
                    payload: source_value,
                },
                TypeTerm::Form {
                    form: target_form,
                    payload: target_value,
                },
            ) => {
                let form = self.decide_form_assignable(source_form, target_form)?;
                if form != Decision::Yes {
                    return Ok(form);
                }

                self.decide_type_relation(TypeRelation::Assignable, *source_value, *target_value)?
            }
            (TypeTerm::Union { elements }, target) => {
                self.decide_all_sources_assignable(elements, target)?
            }
            (source, TypeTerm::Union { elements }) => {
                self.decide_any_target_assignable(source, elements)?
            }
            (TypeTerm::Literal(source), TypeTerm::Literal(target)) => {
                self.decide_dir_type_assignable(source, target)
            }
            (TypeTerm::Array { element: source }, TypeTerm::Array { element: target }) => {
                self.decide_type_relation(TypeRelation::Assignable, *source, *target)?
            }
            (
                TypeTerm::Slice {
                    element: source,
                    is_readonly: source_readonly,
                },
                TypeTerm::Slice {
                    element: target,
                    is_readonly: target_readonly,
                },
            ) => {
                if *source_readonly && !*target_readonly {
                    Decision::No
                } else {
                    self.decide_type_relation(TypeRelation::Assignable, *source, *target)?
                }
            }
            (
                TypeTerm::FixedArray {
                    element: source_element,
                    length: source_length,
                    is_readonly: source_readonly,
                },
                TypeTerm::FixedArray {
                    element: target_element,
                    length: target_length,
                    is_readonly: target_readonly,
                },
            ) => {
                if *source_readonly && !*target_readonly {
                    Decision::No
                } else {
                    let element = self.decide_type_relation(
                        TypeRelation::Assignable,
                        *source_element,
                        *target_element,
                    )?;
                    let length = self.decide_static_relation(
                        StaticRelation::Equal,
                        *source_length,
                        *target_length,
                    )?;

                    element.and(length)
                }
            }
            (
                TypeTerm::FixedArray {
                    element: source,
                    is_readonly: source_readonly,
                    ..
                },
                TypeTerm::Slice {
                    element: target,
                    is_readonly: target_readonly,
                },
            ) => {
                if *source_readonly && !*target_readonly {
                    Decision::No
                } else {
                    self.decide_type_relation(TypeRelation::Assignable, *source, *target)?
                }
            }
            (
                TypeTerm::Tuple {
                    form: source_form,
                    elements: source_elements,
                    is_readonly: source_readonly,
                },
                TypeTerm::Tuple {
                    form: target_form,
                    elements: target_elements,
                    is_readonly: target_readonly,
                },
            ) => {
                if source_form != target_form || (*source_readonly && !*target_readonly) {
                    Decision::No
                } else {
                    self.decide_tuple_elements_assignable(source_elements, target_elements)?
                }
            }
            (TypeTerm::Shape { members: source }, TypeTerm::Shape { members: target }) => {
                self.decide_shape_assignable(source, target)?
            }
            _ => Decision::Undecidable,
        };

        Ok(decision)
    }

    /// Decide exact static equality.
    fn decide_static_equal(
        &self,
        left: &StaticTerm,
        right: &StaticTerm,
    ) -> CompilerResult<Decision> {
        if left == right {
            return Ok(Decision::Yes);
        }

        let decision = match (left, right) {
            (StaticTerm::Variable(left), right) => {
                let Some(left) = self.solved_static_term(*left)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_static_equal(&left, right)?
            }
            (left, StaticTerm::Variable(right)) => {
                let Some(right) = self.solved_static_term(*right)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_static_equal(left, &right)?
            }
            (StaticTerm::Literal(left), StaticTerm::Literal(right)) => {
                self.decide_dir_static_equal(left, right)
            }
            (StaticTerm::Expression(_), _)
            | (_, StaticTerm::Expression(_))
            | (StaticTerm::Member { .. }, _)
            | (_, StaticTerm::Member { .. })
            | (StaticTerm::LifetimeJoin { .. }, _)
            | (_, StaticTerm::LifetimeJoin { .. })
            | (StaticTerm::Intrinsic { .. }, _)
            | (_, StaticTerm::Intrinsic { .. }) => Decision::Undecidable,
        };

        Ok(decision)
    }

    /// Decide exact memory form equality.
    fn decide_form_equal(&self, left: &FormTerm, right: &FormTerm) -> CompilerResult<Decision> {
        if left == right {
            return Ok(Decision::Yes);
        }

        let decision = match (left, right) {
            (
                FormTerm::Borrowed {
                    lifetime: left_lifetime,
                    access: left_access,
                },
                FormTerm::Borrowed {
                    lifetime: right_lifetime,
                    access: right_access,
                },
            ) => {
                let lifetime = self.decide_static_relation(
                    StaticRelation::Equal,
                    *left_lifetime,
                    *right_lifetime,
                )?;
                if lifetime != Decision::Yes {
                    return Ok(lifetime);
                }

                self.decide_static_relation(StaticRelation::Equal, *left_access, *right_access)?
            }
            (FormTerm::Placed { place: left }, FormTerm::Placed { place: right }) => {
                self.decide_static_relation(StaticRelation::Equal, *left, *right)?
            }
            _ => Decision::No,
        };

        Ok(decision)
    }

    /// Decide memory form assignability.
    fn decide_form_assignable(
        &self,
        source: &FormTerm,
        target: &FormTerm,
    ) -> CompilerResult<Decision> {
        if self.decide_form_equal(source, target)? == Decision::Yes {
            return Ok(Decision::Yes);
        }

        let decision = match (source, target) {
            (
                FormTerm::Borrowed { access: source, .. },
                FormTerm::Borrowed { access: target, .. },
            ) => self.decide_access_assignable(*source, *target)?,
            (FormTerm::Placed { place: source }, FormTerm::Placed { place: target }) => {
                self.decide_static_relation(StaticRelation::Equal, *source, *target)?
            }
            (FormTerm::Managed, FormTerm::Managed)
            | (FormTerm::Owned, FormTerm::Owned)
            | (FormTerm::Raw, FormTerm::Raw)
            | (FormTerm::Readonly, FormTerm::Readonly) => Decision::Yes,
            _ => Decision::No,
        };

        Ok(decision)
    }

    /// Decide memory access assignability.
    fn decide_access_assignable(
        &self,
        source: VariableId,
        target: VariableId,
    ) -> CompilerResult<Decision> {
        let Some(source) = self.access_value(source)? else {
            return Ok(Decision::Undecidable);
        };
        let Some(target) = self.access_value(target)? else {
            return Ok(Decision::Undecidable);
        };

        if Self::access_rank(source) >= Self::access_rank(target) {
            Ok(Decision::Yes)
        } else {
            Ok(Decision::No)
        }
    }

    /// Decide exact equality for argument lists.
    fn decide_argument_list_equal(
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
    fn decide_argument_equal(
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

    /// Decide exact equality for type variable lists.
    fn decide_type_variable_list_equal(
        &self,
        left: &[VariableId],
        right: &[VariableId],
    ) -> CompilerResult<Decision> {
        if left.len() != right.len() {
            return Ok(Decision::No);
        }

        let mut decision = Decision::Yes;
        for (left, right) in left.iter().zip(right) {
            decision =
                decision.and(self.decide_type_relation(TypeRelation::Equal, *left, *right)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide exact equality for optional type variables.
    fn decide_optional_type_variable_equal(
        &self,
        left: Option<VariableId>,
        right: Option<VariableId>,
    ) -> CompilerResult<Decision> {
        let decision = match (left, right) {
            (Some(left), Some(right)) => {
                self.decide_type_relation(TypeRelation::Equal, left, right)?
            }
            (None, None) => Decision::Yes,
            _ => Decision::No,
        };

        Ok(decision)
    }

    /// Decide exact equality for tuple element lists.
    fn decide_tuple_elements_equal(
        &self,
        left: &[TupleElementTerm],
        right: &[TupleElementTerm],
    ) -> CompilerResult<Decision> {
        if left.len() != right.len() {
            return Ok(Decision::No);
        }

        let mut decision = Decision::Yes;
        for (left, right) in left.iter().zip(right) {
            decision = decision.and(self.decide_tuple_element_equal(left, right)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide exact equality for one tuple element.
    fn decide_tuple_element_equal(
        &self,
        left: &TupleElementTerm,
        right: &TupleElementTerm,
    ) -> CompilerResult<Decision> {
        if left.label != right.label
            || left.is_optional != right.is_optional
            || left.is_readonly != right.is_readonly
            || left.is_rest != right.is_rest
        {
            return Ok(Decision::No);
        }

        self.decide_type_relation(TypeRelation::Equal, left.ty, right.ty)
    }

    /// Decide tuple element assignability.
    fn decide_tuple_elements_assignable(
        &self,
        source: &[TupleElementTerm],
        target: &[TupleElementTerm],
    ) -> CompilerResult<Decision> {
        if source.len() != target.len() {
            return Ok(Decision::No);
        }

        let mut decision = Decision::Yes;
        for (source, target) in source.iter().zip(target) {
            decision = decision.and(self.decide_tuple_element_assignable(source, target)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide one tuple element assignability.
    fn decide_tuple_element_assignable(
        &self,
        source: &TupleElementTerm,
        target: &TupleElementTerm,
    ) -> CompilerResult<Decision> {
        if source.is_rest != target.is_rest
            || source.is_readonly && !target.is_readonly
            || source.is_optional && !target.is_optional
        {
            return Ok(Decision::No);
        }

        self.decide_type_relation(TypeRelation::Assignable, source.ty, target.ty)
    }

    /// Decide exact equality for shape members.
    fn decide_shape_members_equal(
        &self,
        left: &[ShapeMemberTerm],
        right: &[ShapeMemberTerm],
    ) -> CompilerResult<Decision> {
        if left.len() != right.len() {
            return Ok(Decision::No);
        }

        let mut decision = Decision::Yes;
        for (left, right) in left.iter().zip(right) {
            decision = decision.and(self.decide_shape_member_equal(left, right)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
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

    /// Decide structural shape assignability.
    fn decide_shape_assignable(
        &self,
        source: &[ShapeMemberTerm],
        target: &[ShapeMemberTerm],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;
        for target in target {
            decision = decision.and(self.decide_shape_member_assignable(source, target)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

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

    /// Decide exact equality for function terms.
    fn decide_function_equal(
        &self,
        left: &FunctionTerm,
        right: &FunctionTerm,
    ) -> CompilerResult<Decision> {
        if left.asynchrony != right.asynchrony || left.is_generator != right.is_generator {
            return Ok(Decision::No);
        }

        let generics = self
            .decide_type_variable_list_equal(&left.generic_parameters, &right.generic_parameters)?;
        let this =
            self.decide_optional_type_variable_equal(left.this_parameter, right.this_parameter)?;
        let parameters =
            self.decide_type_variable_list_equal(&left.parameters, &right.parameters)?;
        let return_type =
            self.decide_optional_type_variable_equal(left.return_type, right.return_type)?;

        Ok(generics.and(this).and(parameters).and(return_type))
    }

    /// Return one solved access value.
    fn access_value(&self, variable: VariableId) -> CompilerResult<Option<dir::Access>> {
        let Some(term) = self.solved_static_term(variable)? else {
            return Ok(None);
        };
        let access = match term {
            StaticTerm::Literal(dir::StaticTerm::Access { access }) => Some(access),
            _ => None,
        };

        Ok(access)
    }

    /// Return one access capability rank.
    fn access_rank(access: dir::Access) -> u8 {
        match access {
            dir::Access::Readonly => 0,
            dir::Access::Mutable => 1,
            dir::Access::Exclusive => 2,
        }
    }

    /// Decide exact DIR type equality.
    fn decide_dir_type_equal(&self, left: &dir::Type, right: &dir::Type) -> Decision {
        if left == right {
            Decision::Yes
        } else {
            Decision::No
        }
    }

    /// Decide DIR type assignability.
    fn decide_dir_type_assignable(&self, source: &dir::Type, target: &dir::Type) -> Decision {
        if source == target {
            return Decision::Yes;
        }

        match (source, target) {
            (dir::Type::Error, _) | (_, dir::Type::Error) => Decision::Yes,
            (dir::Type::Any, _) | (_, dir::Type::Any) => Decision::Yes,
            (dir::Type::Never, _) => Decision::Yes,
            (_, dir::Type::Unknown) => Decision::Yes,
            (dir::Type::Unknown, _) => Decision::No,
            (dir::Type::Literal(literal), target) => {
                self.decide_literal_assignable(literal, target)
            }
            _ => Decision::No,
        }
    }

    /// Decide literal widening assignability.
    fn decide_literal_assignable(
        &self,
        literal: &dir::ScalarLiteral,
        target: &dir::Type,
    ) -> Decision {
        match (literal, target) {
            (dir::ScalarLiteral::Null, dir::Type::Null) => Decision::Yes,
            (dir::ScalarLiteral::Boolean(_), dir::Type::Primitive(dir::PrimitiveType::Boolean)) => {
                Decision::Yes
            }
            (
                dir::ScalarLiteral::Character(_),
                dir::Type::Primitive(dir::PrimitiveType::Character),
            ) => Decision::Yes,
            (dir::ScalarLiteral::String(_), dir::Type::Primitive(dir::PrimitiveType::String)) => {
                Decision::Yes
            }
            (dir::ScalarLiteral::Bigint(_), dir::Type::Primitive(dir::PrimitiveType::Bigint)) => {
                Decision::Yes
            }
            (
                dir::ScalarLiteral::Integer(_),
                dir::Type::Primitive(dir::PrimitiveType::Integer(_)),
            ) => Decision::Yes,
            (dir::ScalarLiteral::Float(_), dir::Type::Primitive(dir::PrimitiveType::Float(_))) => {
                Decision::Yes
            }
            (dir::ScalarLiteral::RegexString { .. }, dir::Type::Object) => Decision::Yes,
            _ => Decision::No,
        }
    }

    /// Decide whether all source union elements assign to a target.
    fn decide_all_sources_assignable(
        &self,
        sources: &[VariableId],
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        for source in sources {
            let Some(source) = self.solved_type_term(*source)? else {
                return Ok(Decision::Undecidable);
            };
            let decision = self.decide_type_assignable(&source, target)?;
            if decision != Decision::Yes {
                return Ok(decision);
            }
        }

        Ok(Decision::Yes)
    }

    /// Decide whether one source assigns to any target union element.
    fn decide_any_target_assignable(
        &self,
        source: &TypeTerm,
        targets: &[VariableId],
    ) -> CompilerResult<Decision> {
        let mut has_unknown = false;

        // accept the first matching target
        for target in targets {
            let Some(target) = self.solved_type_term(*target)? else {
                has_unknown = true;
                continue;
            };
            match self.decide_type_assignable(source, &target)? {
                Decision::Yes => return Ok(Decision::Yes),
                Decision::Undecidable => has_unknown = true,
                Decision::No => {}
            }
        }

        if has_unknown {
            Ok(Decision::Undecidable)
        } else {
            Ok(Decision::No)
        }
    }

    /// Decide exact DIR static equality.
    fn decide_dir_static_equal(&self, left: &dir::StaticTerm, right: &dir::StaticTerm) -> Decision {
        if left == right {
            Decision::Yes
        } else {
            Decision::No
        }
    }
}
