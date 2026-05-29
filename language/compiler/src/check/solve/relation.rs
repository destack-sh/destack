use crate::check::{
    CheckState, Origin, StaticOperand, StaticTerm, TypeOperand, TypeRelation, TypeTerm,
};
use crate::{CompilerError, CompilerResult};

use super::Progress;

/// Maximum number of local type reduction steps before reporting complexity.
const MAX_TYPE_REDUCTION_STEPS: usize = 64;

impl CheckState<'_> {
    /// Solve one type relation.
    pub(in crate::check) fn solve_type_relation(
        &mut self,
        origin: Origin,
        relation: TypeRelation,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
    ) -> CompilerResult<Progress> {
        let left = left.into();
        let right = right.into();

        match relation {
            TypeRelation::Equal => self.solve_type_equality(origin, left, right),
            TypeRelation::Assignable | TypeRelation::Castable => {
                self.solve_contextual_type_assignability(origin, left, right)
            }
            TypeRelation::Satisfies | TypeRelation::Extends | TypeRelation::Implements => {
                self.solve_type_assignability(origin, left, right)
            }
        }
    }

    /// Solve one type equality relation.
    pub(in crate::check) fn solve_type_equality(
        &mut self,
        origin: Origin,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
    ) -> CompilerResult<Progress> {
        let left = left.into();
        let right = right.into();
        let bounds = self.add_equal_bounds(left, right)?;
        let left_value = self.reduce_type_operand(origin, left)?;
        let right_value = self.reduce_type_operand(origin, right)?;

        let progress = match (left_value, right_value) {
            (Some(term), None) => match right.variable() {
                Some(right) => self.solve_type_variable(right, term),
                None => Ok(Progress::Unchanged),
            },
            (None, Some(term)) => match left.variable() {
                Some(left) => self.solve_type_variable(left, term),
                None => Ok(Progress::Unchanged),
            },
            (Some(left), Some(right)) => self.constrain_solved_type_equal(origin, &left, &right),
            (None, None) => Ok(Progress::Unchanged),
        }?;

        Ok(bounds.merge(progress))
    }

    /// Solve one assignability relation.
    pub(in crate::check) fn solve_contextual_type_assignability(
        &mut self,
        origin: Origin,
        source: impl Into<TypeOperand>,
        target: impl Into<TypeOperand>,
    ) -> CompilerResult<Progress> {
        let source = source.into();
        let target = target.into();
        let bounds = self.add_assignable_bounds(source, target)?;
        let target_value = self.reduce_type_operand(origin, target)?;
        let expected = match &target_value {
            Some(target_term) => {
                self.expect_type_operand_assignability(origin, source, target, target_term)?
            }
            None => Progress::Unchanged,
        };
        let source_value = self.reduce_type_operand(origin, source)?;

        let progress = match (source_value, target_value) {
            (Some(source_term), Some(target_term)) => {
                Ok::<Progress, CompilerError>(expected.merge(
                    self.constrain_solved_type_assignable(origin, &source_term, &target_term)?,
                ))
            }
            (Some(_), None) => match target.variable() {
                Some(target) => self.solve_bound_variable(target),
                None => Ok(Progress::Unchanged),
            },
            (None, Some(_)) | (None, None) => Ok(Progress::Unchanged),
        }?;

        Ok(bounds.merge(progress))
    }

    /// Solve one assignability relation without contextual backpressure.
    pub(in crate::check) fn solve_type_assignability(
        &mut self,
        origin: Origin,
        source: impl Into<TypeOperand>,
        target: impl Into<TypeOperand>,
    ) -> CompilerResult<Progress> {
        let source = source.into();
        let target = target.into();
        let bounds = self.add_assignable_bounds(source, target)?;
        let target_value = self.reduce_type_operand(origin, target)?;
        let source_value = self.reduce_type_operand(origin, source)?;

        let progress = match (source_value, target_value) {
            (Some(source_term), Some(target_term)) => {
                self.constrain_solved_type_assignable(origin, &source_term, &target_term)
            }
            (Some(_), None) => match target.variable() {
                Some(target) => self.solve_bound_variable(target),
                None => Ok(Progress::Unchanged),
            },
            (None, Some(_)) | (None, None) => Ok(Progress::Unchanged),
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

        for _ in 0..MAX_TYPE_REDUCTION_STEPS {
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

        Err(self.type_too_complex_error(origin).into())
    }

    /// Solve one static equality relation.
    pub(in crate::check) fn solve_static_equality(
        &mut self,
        left: impl Into<StaticOperand>,
        right: impl Into<StaticOperand>,
    ) -> CompilerResult<Progress> {
        let left = left.into();
        let right = right.into();
        let bounds = self.add_equal_static_bounds(left, right)?;
        let left_value = self.static_operand_term(left)?;
        let right_value = self.static_operand_term(right)?;

        let progress = match (left_value, right_value) {
            (Some(term), None) => match right.variable() {
                Some(right) => self.solve_static_variable(right, term),
                None => Ok(Progress::Unchanged),
            },
            (None, Some(term)) => match left.variable() {
                Some(left) => self.solve_static_variable(left, term),
                None => Ok(Progress::Unchanged),
            },
            _ => Ok(Progress::Unchanged),
        }?;

        Ok(bounds.merge(progress))
    }

    /// Solve one static assignability relation.
    pub(in crate::check) fn solve_static_assignability(
        &mut self,
        source: impl Into<StaticOperand>,
        target: impl Into<StaticOperand>,
    ) -> CompilerResult<Progress> {
        let source = source.into();
        let target = target.into();
        let bounds = self.add_assignable_static_bounds(source, target)?;
        let source_value = self.static_operand_term(source)?;
        let target_value = self.static_operand_term(target)?;

        let progress = match (source_value, target_value) {
            (Some(_), None) => match target.variable() {
                Some(target) => self.solve_bound_variable(target),
                None => Ok(Progress::Unchanged),
            },
            (None, Some(_)) => match source.variable() {
                Some(source) => self.solve_bound_variable(source),
                None => Ok(Progress::Unchanged),
            },
            (Some(_), Some(_)) => Ok(Progress::Unchanged),
            (None, None) => Ok(Progress::Unchanged),
        }?;

        Ok(bounds.merge(progress))
    }

    /// Return the solved term for one type operand when available.
    pub(in crate::check) fn type_operand_term(
        &self,
        operand: TypeOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match operand {
            TypeOperand::Variable(variable) => self.solved_type_term(variable)?,
            TypeOperand::Term(term) => Some(self.terms.get(term).clone()),
        };

        Ok(term)
    }

    /// Return the solved term for one static operand when available.
    pub(in crate::check) fn static_operand_term(
        &self,
        operand: StaticOperand,
    ) -> CompilerResult<Option<StaticTerm>> {
        let term = match operand {
            StaticOperand::Variable(variable) => self.solved_static_term(variable)?,
            StaticOperand::Term(term) => Some(self.terms.get(term).clone()),
        };

        Ok(term)
    }

    /// Add equality bounds between two type operands.
    fn add_equal_bounds(
        &mut self,
        left: TypeOperand,
        right: TypeOperand,
    ) -> CompilerResult<Progress> {
        let left_to_right = self.add_assignable_bounds(left, right)?;
        let right_to_left = self.add_assignable_bounds(right, left)?;

        Ok(left_to_right.merge(right_to_left))
    }

    /// Add equality bounds between two static operands.
    fn add_equal_static_bounds(
        &mut self,
        left: StaticOperand,
        right: StaticOperand,
    ) -> CompilerResult<Progress> {
        let left_to_right = self.add_assignable_static_bounds(left, right)?;
        let right_to_left = self.add_assignable_static_bounds(right, left)?;

        Ok(left_to_right.merge(right_to_left))
    }

    /// Add assignability bounds between two type operands.
    fn add_assignable_bounds(
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
            && self.add_lower_type_bound(target, source)?
        {
            progress = progress.merge(Progress::changed(target));
        }

        // wake users of the constrained source
        if let Some(source) = source.variable()
            && self.add_upper_type_bound(source, target)?
        {
            progress = progress.merge(Progress::changed(source));
        }

        Ok(progress)
    }

    /// Add assignability bounds between two static operands.
    fn add_assignable_static_bounds(
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
            && self.add_lower_static_bound(target, source)?
        {
            progress = progress.merge(Progress::changed(target));
        }

        // wake users of the constrained source
        if let Some(source) = source.variable()
            && self.add_upper_static_bound(source, target)?
        {
            progress = progress.merge(Progress::changed(source));
        }

        Ok(progress)
    }
}
