use crate::check::{CheckComponentState, StaticRelation, TypeRelation, VariableId};
use crate::{CompilerError, CompilerResult};

use super::Progress;

impl CheckComponentState<'_> {
    /// Solve one type relation.
    pub(in crate::check) fn solve_type_relation(
        &mut self,
        relation: TypeRelation,
        left: VariableId,
        right: VariableId,
    ) -> CompilerResult<Progress> {
        match relation {
            TypeRelation::Equal => self.solve_type_equality(left, right),
            TypeRelation::Assignable => self.solve_type_assignability(left, right),
            TypeRelation::Castable
            | TypeRelation::Satisfies
            | TypeRelation::Extends
            | TypeRelation::Implements => Ok(Progress::Unchanged),
        }
    }

    /// Solve one type equality relation.
    pub(in crate::check) fn solve_type_equality(
        &mut self,
        left: VariableId,
        right: VariableId,
    ) -> CompilerResult<Progress> {
        let bounds = self.add_equal_bounds(left, right)?;
        let left_value = self.solved_type_term(left)?;
        let right_value = self.solved_type_term(right)?;

        let progress = match (left_value, right_value) {
            (Some(term), None) => self.solve_type_variable(right, term),
            (None, Some(term)) => self.solve_type_variable(left, term),
            (Some(left), Some(right)) => self.relate_solved_type_equal(&left, &right),
            (None, None) => Ok(Progress::Unchanged),
        }?;

        Ok(bounds.merge(progress))
    }

    /// Solve one assignability relation.
    pub(in crate::check) fn solve_type_assignability(
        &mut self,
        source: VariableId,
        target: VariableId,
    ) -> CompilerResult<Progress> {
        let bounds = self.add_assignable_bounds(source, target)?;
        let source_value = self.solved_type_term(source)?;
        let target_value = self.solved_type_term(target)?;

        let progress = match (source_value, target_value) {
            (Some(source_term), Some(target_term)) => {
                let expected =
                    self.propagate_literal_type_expectation(source, &source_term, &target_term)?;
                let relation = self.relate_solved_type_assignable(&source_term, &target_term)?;

                Ok::<Progress, CompilerError>(expected.merge(relation))
            }
            (Some(_), None) | (None, Some(_)) | (None, None) => Ok(Progress::Unchanged),
        }?;

        Ok(bounds.merge(progress))
    }

    /// Solve one static equality relation.
    pub(in crate::check) fn solve_static_equality(
        &mut self,
        left: VariableId,
        right: VariableId,
    ) -> CompilerResult<Progress> {
        let bounds = self.add_equal_bounds(left, right)?;
        let left_value = self.solved_static_term(left)?;
        let right_value = self.solved_static_term(right)?;

        let progress = match (left_value, right_value) {
            (Some(term), None) => self.solve_static_variable(right, term),
            (None, Some(term)) => self.solve_static_variable(left, term),
            _ => Ok(Progress::Unchanged),
        }?;

        Ok(bounds.merge(progress))
    }

    /// Solve one static relation.
    pub(in crate::check) fn solve_static_relation(
        &mut self,
        relation: StaticRelation,
        left: VariableId,
        right: VariableId,
    ) -> CompilerResult<Progress> {
        match relation {
            StaticRelation::Equal => self.solve_static_equality(left, right),
        }
    }

    /// Record equality bounds between two variables.
    fn add_equal_bounds(
        &mut self,
        left: VariableId,
        right: VariableId,
    ) -> CompilerResult<Progress> {
        let left_to_right = self.add_assignable_bounds(left, right)?;
        let right_to_left = self.add_assignable_bounds(right, left)?;

        Ok(left_to_right.merge(right_to_left))
    }

    /// Record assignability bounds between two variables.
    fn add_assignable_bounds(
        &mut self,
        source: VariableId,
        target: VariableId,
    ) -> CompilerResult<Progress> {
        let lower_changed = self
            .module_mut(target.module)?
            .add_lower_bound(target, source);
        let upper_changed = self
            .module_mut(source.module)?
            .add_upper_bound(source, target);
        let mut progress = Progress::Unchanged;

        // wake users of the constrained target
        if lower_changed {
            progress = progress.merge(Progress::changed(target));
        }

        // wake users of the constrained source
        if upper_changed {
            progress = progress.merge(Progress::changed(source));
        }

        Ok(progress)
    }
}
