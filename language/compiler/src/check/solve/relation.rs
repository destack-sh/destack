use crate::check::{
    CheckState, StaticOperand, StaticRelation, StaticTerm, TypeOperand, TypeRelation, TypeTerm,
};
use crate::{CompilerError, CompilerResult};

use super::Progress;

impl CheckState<'_> {
    /// Solve one type relation.
    pub(in crate::check) fn solve_type_relation(
        &mut self,
        relation: TypeRelation,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
    ) -> CompilerResult<Progress> {
        let left = left.into();
        let right = right.into();

        match relation {
            TypeRelation::Equal => self.solve_type_equality(left, right),
            TypeRelation::Assignable => self.solve_type_assignability(left, right),
            TypeRelation::Satisfies | TypeRelation::Extends | TypeRelation::Implements => {
                self.solve_type_assignability(left, right)
            }
            TypeRelation::Castable => Ok(Progress::Unchanged),
        }
    }

    /// Solve one type equality relation.
    pub(in crate::check) fn solve_type_equality(
        &mut self,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
    ) -> CompilerResult<Progress> {
        let left = left.into();
        let right = right.into();
        let bounds = self.add_equal_bounds(left, right)?;
        let left_value = self.type_operand_term(left)?;
        let right_value = self.type_operand_term(right)?;

        let progress = match (left_value, right_value) {
            (Some(term), None) => match right.variable() {
                Some(right) => self.solve_type_variable(right, term),
                None => Ok(Progress::Unchanged),
            },
            (None, Some(term)) => match left.variable() {
                Some(left) => self.solve_type_variable(left, term),
                None => Ok(Progress::Unchanged),
            },
            (Some(left), Some(right)) => self.constrain_solved_type_equal(&left, &right),
            (None, None) => Ok(Progress::Unchanged),
        }?;

        Ok(bounds.merge(progress))
    }

    /// Solve one assignability relation.
    pub(in crate::check) fn solve_type_assignability(
        &mut self,
        source: impl Into<TypeOperand>,
        target: impl Into<TypeOperand>,
    ) -> CompilerResult<Progress> {
        let source = source.into();
        let target = target.into();
        let bounds = self.add_assignable_bounds(source, target)?;
        let source_value = self.type_operand_term(source)?;
        let target_value = self.type_operand_term(target)?;

        let progress = match (source_value, target_value) {
            (Some(source_term), Some(target_term)) => {
                let expected = match source.variable() {
                    Some(source) => self.expect_literal_term(source, &source_term, &target_term)?,
                    None => Progress::Unchanged,
                };
                let relation = self.constrain_solved_type_assignable(&source_term, &target_term)?;

                Ok::<Progress, CompilerError>(expected.merge(relation))
            }
            (Some(_), None) | (None, Some(_)) | (None, None) => Ok(Progress::Unchanged),
        }?;

        Ok(bounds.merge(progress))
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

    /// Solve one static relation.
    pub(in crate::check) fn solve_static_relation(
        &mut self,
        relation: StaticRelation,
        left: StaticOperand,
        right: StaticOperand,
    ) -> CompilerResult<Progress> {
        match relation {
            StaticRelation::Equal => self.solve_static_equality(left, right),
        }
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

    /// Record equality bounds between two type operands.
    fn add_equal_bounds(
        &mut self,
        left: TypeOperand,
        right: TypeOperand,
    ) -> CompilerResult<Progress> {
        let left_to_right = self.add_assignable_bounds(left, right)?;
        let right_to_left = self.add_assignable_bounds(right, left)?;

        Ok(left_to_right.merge(right_to_left))
    }

    /// Record equality bounds between two static operands.
    fn add_equal_static_bounds(
        &mut self,
        left: StaticOperand,
        right: StaticOperand,
    ) -> CompilerResult<Progress> {
        let left_to_right = self.add_assignable_static_bounds(left, right)?;
        let right_to_left = self.add_assignable_static_bounds(right, left)?;

        Ok(left_to_right.merge(right_to_left))
    }

    /// Record assignability bounds between two type operands.
    fn add_assignable_bounds(
        &mut self,
        source: TypeOperand,
        target: TypeOperand,
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // wake users of the constrained target
        if let Some(target) = target.variable()
            && self.add_lower_type_bound(target, source)
        {
            progress = progress.merge(Progress::changed(target));
        }

        // wake users of the constrained source
        if let Some(source) = source.variable()
            && self.add_upper_type_bound(source, target)
        {
            progress = progress.merge(Progress::changed(source));
        }

        Ok(progress)
    }

    /// Record assignability bounds between two static operands.
    fn add_assignable_static_bounds(
        &mut self,
        source: StaticOperand,
        target: StaticOperand,
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // wake users of the constrained target
        if let Some(target) = target.variable()
            && self.add_lower_static_bound(target, source)
        {
            progress = progress.merge(Progress::changed(target));
        }

        // wake users of the constrained source
        if let Some(source) = source.variable()
            && self.add_upper_static_bound(source, target)
        {
            progress = progress.merge(Progress::changed(source));
        }

        Ok(progress)
    }
}
