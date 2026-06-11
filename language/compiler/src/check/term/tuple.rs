use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, SubstitutionSet, TypeOperand, TypeRelation};

/// Tuple element payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct TupleElement {
    /// The optional label for the element.
    pub(in crate::check) label: Option<dir::StringId>,
    /// The element type.
    pub(in crate::check) ty: TypeOperand,
    /// Whether the element is optional.
    pub(in crate::check) is_optional: bool,
    /// Whether the element is readonly.
    pub(in crate::check) is_readonly: bool,
    /// Whether the element is a rest element.
    pub(in crate::check) is_rest: bool,
}

impl TupleElement {
    /// Substitute generic arguments through this tuple element.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        Ok(Self {
            label: self.label,
            ty: state.substitute_type_operand(module, substitution, self.ty)?,
            is_optional: self.is_optional,
            is_readonly: self.is_readonly,
            is_rest: self.is_rest,
        })
    }
}

impl CheckState<'_> {
    /// Substitute generic arguments through tuple elements.
    pub(in crate::check) fn substitute_tuple_elements(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        elements: &[TupleElement],
    ) -> CompilerResult<Vec<TupleElement>> {
        elements
            .iter()
            .map(|element| element.substitute(module, substitution, self))
            .collect()
    }

    /// Decide exact equality for one tuple element.
    pub(in crate::check) fn decide_tuple_element_equal(
        &mut self,
        left: &TupleElement,
        right: &TupleElement,
    ) -> CompilerResult<Answer<bool>> {
        if left.label != right.label
            || left.is_optional != right.is_optional
            || left.is_readonly != right.is_readonly
            || left.is_rest != right.is_rest
        {
            return Ok(Answer::Ready(false));
        }

        self.decide_type_relation(TypeRelation::Equal, left.ty, right.ty)
    }

    /// Decide one tuple element assignability.
    pub(in crate::check) fn decide_tuple_element_assignable(
        &mut self,
        source: &TupleElement,
        target: &TupleElement,
    ) -> CompilerResult<Answer<bool>> {
        if source.is_rest != target.is_rest
            || source.is_readonly && !target.is_readonly
            || source.is_optional && !target.is_optional
        {
            return Ok(Answer::Ready(false));
        }

        self.decide_type_relation(TypeRelation::Assignable, source.ty, target.ty)
    }
}
