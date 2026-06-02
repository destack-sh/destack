use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, Origin, Progress, Substitution, TypeOperand, TypeRelation,
};

/// Tuple element payload.
#[derive(Debug, Clone, PartialEq)]
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
        substitution: Substitution<'_>,
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
        substitution: Substitution<'_>,
        elements: &[TupleElement],
    ) -> CompilerResult<Vec<TupleElement>> {
        elements
            .iter()
            .map(|element| element.substitute(module, substitution, self))
            .collect()
    }

    /// Decide exact equality for tuple element lists.
    pub(in crate::check) fn decide_tuple_elements_equal(
        &self,
        left: &[TupleElement],
        right: &[TupleElement],
    ) -> CompilerResult<Decision> {
        if left.len() != right.len() {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        // compare matching element slots
        for (left, right) in left.iter().zip(right) {
            decision = decision.and(self.decide_tuple_element_equal(left, right)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide tuple element assignability.
    pub(in crate::check) fn decide_tuple_elements_assignable(
        &self,
        source: &[TupleElement],
        target: &[TupleElement],
    ) -> CompilerResult<Decision> {
        if source.len() != target.len() {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        // compare matching element slots
        for (source, target) in source.iter().zip(target) {
            decision = decision.and(self.decide_tuple_element_assignable(source, target)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Relate matching tuple elements by equality.
    pub(in crate::check) fn constrain_tuple_elements_equal(
        &mut self,
        origin: Origin,
        left: &[TupleElement],
        right: &[TupleElement],
    ) -> CompilerResult<Progress> {
        if left.len() != right.len() {
            return Ok(Progress::Unchanged);
        }
        let mut progress = Progress::Unchanged;

        // constrain each matching element
        for (left, right) in left.iter().zip(right) {
            progress = progress.merge(self.relate_type_equality(origin, left.ty, right.ty)?);
        }

        Ok(progress)
    }

    /// Relate matching tuple elements by assignability.
    pub(in crate::check) fn constrain_tuple_elements_assignable(
        &mut self,
        origin: Origin,
        source: &[TupleElement],
        target: &[TupleElement],
    ) -> CompilerResult<Progress> {
        if source.len() != target.len() {
            return Ok(Progress::Unchanged);
        }
        let mut progress = Progress::Unchanged;

        // constrain each matching element
        for (source, target) in source.iter().zip(target) {
            progress =
                progress.merge(self.relate_type_assignability(origin, source.ty, target.ty)?);
        }

        Ok(progress)
    }

    /// Expect tuple elements to satisfy expected elements.
    pub(in crate::check) fn expect_tuple_element_terms(
        &mut self,
        origin: Origin,
        elements: &[TupleElement],
        targets: &[TupleElement],
    ) -> CompilerResult<Progress> {
        if elements.len() != targets.len() {
            return Ok(Progress::Unchanged);
        }
        let mut progress = Progress::Unchanged;

        // push each expected element type
        for (element, target) in elements.iter().zip(targets) {
            progress = progress
                .merge(self.relate_contextual_type_assignability(origin, element.ty, target.ty)?);
        }

        Ok(progress)
    }

    /// Decide exact equality for one tuple element.
    fn decide_tuple_element_equal(
        &self,
        left: &TupleElement,
        right: &TupleElement,
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

    /// Decide one tuple element assignability.
    fn decide_tuple_element_assignable(
        &self,
        source: &TupleElement,
        target: &TupleElement,
    ) -> CompilerResult<Decision> {
        if source.is_rest != target.is_rest
            || source.is_readonly && !target.is_readonly
            || source.is_optional && !target.is_optional
        {
            return Ok(Decision::No);
        }

        self.decide_type_relation(TypeRelation::Assignable, source.ty, target.ty)
    }
}
