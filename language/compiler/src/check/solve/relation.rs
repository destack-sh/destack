use crate::CompilerResult;
use crate::check::{
    CheckState, Origin, StaticOperand, StaticTerm, TypeOperand, TypeRelation, TypeTerm,
};

use super::Progress;

impl CheckState<'_> {
    /// Relate one type relation.
    pub(in crate::check) fn relate_type_relation(
        &mut self,
        origin: Origin,
        relation: TypeRelation,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
    ) -> CompilerResult<Progress> {
        let left = left.into();
        let right = right.into();

        match relation {
            TypeRelation::Equal => self.relate_type_equality(origin, left, right),
            TypeRelation::Assignable | TypeRelation::Castable => {
                self.relate_contextual_type_assignability(origin, left, right)
            }
            TypeRelation::Satisfies | TypeRelation::Extends | TypeRelation::Implements => {
                self.relate_type_assignability(origin, left, right)
            }
        }
    }

    /// Relate one type equality relation.
    pub(in crate::check) fn relate_type_equality(
        &mut self,
        origin: Origin,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
    ) -> CompilerResult<Progress> {
        let left = left.into();
        let right = right.into();
        let bounds = self.insert_equal_type_bounds(left, right)?;
        let left_value = self.reduce_type_operand(origin, left)?;
        let right_value = self.reduce_type_operand(origin, right)?;

        let progress = match (left_value, right_value) {
            // push left type into right operand
            (Some(term), None) => match right.variable() {
                Some(right) => self.expect_type_term(origin, right, &term),
                None => Ok(Progress::Unchanged),
            },
            // push right type into left operand
            (None, Some(term)) => match left.variable() {
                Some(left) => self.expect_type_term(origin, left, &term),
                None => Ok(Progress::Unchanged),
            },
            // validate reduced terms
            (Some(left), Some(right)) => self.constrain_solved_type_equal(origin, &left, &right),
            // wait for operands
            (None, None) => Ok(Progress::Unchanged),
        }?;

        Ok(bounds.merge(progress))
    }

    /// Relate one contextual assignability relation.
    pub(in crate::check) fn relate_contextual_type_assignability(
        &mut self,
        origin: Origin,
        source: impl Into<TypeOperand>,
        target: impl Into<TypeOperand>,
    ) -> CompilerResult<Progress> {
        let source = source.into();
        let target = target.into();
        let bounds = self.insert_assignable_type_bounds(source, target)?;
        let target_value = self.reduce_type_operand(origin, target)?;
        let expected = match &target_value {
            Some(target_term) => {
                self.expect_type_operand_assignability(origin, source, target, target_term)?
            }
            None => Progress::Unchanged,
        };
        let source_value = self.reduce_type_operand(origin, source)?;

        let progress = match (source_value, target_value) {
            // validate reduced terms
            (Some(source_term), Some(target_term)) => {
                let constraint =
                    self.constrain_solved_type_assignable(origin, &source_term, &target_term)?;

                expected.merge(constraint)
            }
            // wait for operands
            (Some(_), None) | (None, Some(_)) | (None, None) => Progress::Unchanged,
        };

        Ok(bounds.merge(progress))
    }

    /// Relate one assignability relation without contextual backpressure.
    pub(in crate::check) fn relate_type_assignability(
        &mut self,
        origin: Origin,
        source: impl Into<TypeOperand>,
        target: impl Into<TypeOperand>,
    ) -> CompilerResult<Progress> {
        let source = source.into();
        let target = target.into();
        let bounds = self.insert_assignable_type_bounds(source, target)?;
        let target_value = self.reduce_type_operand(origin, target)?;
        let source_value = self.reduce_type_operand(origin, source)?;

        let progress = match (source_value, target_value) {
            (Some(source_term), Some(target_term)) => {
                self.constrain_solved_type_assignable(origin, &source_term, &target_term)
            }
            (Some(_), None) | (None, Some(_)) | (None, None) => Ok(Progress::Unchanged),
        }?;

        Ok(bounds.merge(progress))
    }

    /// Return the reduced term for one type operand when available.
    pub(in crate::check) fn reduce_type_operand(
        &mut self,
        origin: Origin,
        operand: TypeOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(term) = self.type_operand_term(operand)? else {
            return Ok(None);
        };

        self.reduce_type_operand_term(origin, term)
    }

    /// Return the fixed point reduction for one type term.
    pub(in crate::check) fn reduce_type_operand_term(
        &mut self,
        origin: Origin,
        mut term: TypeTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let mut seen = Vec::new();

        loop {
            let reduction = self.reduce_type_term(origin, &term)?;
            let Some(next) = reduction.value else {
                return Ok(None);
            };
            if next == term {
                return Ok(Some(term));
            }
            if seen.contains(&next) {
                return Err(self.circular_type_error(origin).into());
            }

            seen.push(term);
            term = next;
        }
    }

    /// Relate one static equality relation.
    pub(in crate::check) fn relate_static_equality(
        &mut self,
        left: impl Into<StaticOperand>,
        right: impl Into<StaticOperand>,
    ) -> CompilerResult<Progress> {
        let left = left.into();
        let right = right.into();
        let bounds = self.insert_equal_static_bounds(left, right)?;

        Ok(bounds)
    }

    /// Relate one static assignability relation.
    pub(in crate::check) fn relate_static_assignability(
        &mut self,
        source: impl Into<StaticOperand>,
        target: impl Into<StaticOperand>,
    ) -> CompilerResult<Progress> {
        let source = source.into();
        let target = target.into();
        let bounds = self.insert_assignable_static_bounds(source, target)?;

        Ok(bounds)
    }

    /// Return the solved term for one type operand when available.
    pub(in crate::check) fn type_operand_term(
        &self,
        operand: TypeOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match operand {
            TypeOperand::Variable(variable) => self.type_solution(variable)?,
            TypeOperand::Term(term) => Some(self.inference.term(term).clone()),
            TypeOperand::Type(ty) => Some(TypeTerm::Type(ty)),
        };

        Ok(term)
    }

    /// Return the solved term for one static operand when available.
    pub(in crate::check) fn static_operand_term(
        &self,
        operand: StaticOperand,
    ) -> CompilerResult<Option<StaticTerm>> {
        let term = match operand {
            StaticOperand::Variable(variable) => self.static_solution(variable)?,
            StaticOperand::Term(term) => Some(self.inference.term(term).clone()),
            StaticOperand::Static(value) => Some(StaticTerm::Static(value)),
        };

        Ok(term)
    }

    /// Insert equality bounds between two type operands.
    fn insert_equal_type_bounds(
        &mut self,
        left: TypeOperand,
        right: TypeOperand,
    ) -> CompilerResult<Progress> {
        let left_to_right = self.insert_assignable_type_bounds(left, right)?;
        let right_to_left = self.insert_assignable_type_bounds(right, left)?;

        Ok(left_to_right.merge(right_to_left))
    }

    /// Insert equality bounds between two static operands.
    fn insert_equal_static_bounds(
        &mut self,
        left: StaticOperand,
        right: StaticOperand,
    ) -> CompilerResult<Progress> {
        let left_to_right = self.insert_assignable_static_bounds(left, right)?;
        let right_to_left = self.insert_assignable_static_bounds(right, left)?;

        Ok(left_to_right.merge(right_to_left))
    }

    /// Insert assignability bounds between two type operands.
    fn insert_assignable_type_bounds(
        &mut self,
        source: TypeOperand,
        target: TypeOperand,
    ) -> CompilerResult<Progress> {
        if source == target {
            return Ok(Progress::Unchanged);
        }
        let mut progress = Progress::Unchanged;

        // wake users of the constrained target
        if let Some(target) = target.variable()
            && self.insert_lower_type_bound(target, source)?
        {
            progress = progress.merge(Progress::changed(target));
        }

        // wake users of the constrained source
        if let Some(source) = source.variable()
            && self.insert_upper_type_bound(source, target)?
        {
            progress = progress.merge(Progress::changed(source));
        }

        Ok(progress)
    }

    /// Insert assignability bounds between two static operands.
    fn insert_assignable_static_bounds(
        &mut self,
        source: StaticOperand,
        target: StaticOperand,
    ) -> CompilerResult<Progress> {
        if source == target {
            return Ok(Progress::Unchanged);
        }
        let mut progress = Progress::Unchanged;

        // wake users of the constrained target
        if let Some(target) = target.variable()
            && self.insert_lower_static_bound(target, source)?
        {
            progress = progress.merge(Progress::changed(target));
        }

        // wake users of the constrained source
        if let Some(source) = source.variable()
            && self.insert_upper_static_bound(source, target)?
        {
            progress = progress.merge(Progress::changed(source));
        }

        Ok(progress)
    }
}
