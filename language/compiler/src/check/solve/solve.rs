use crate::CompilerResult;
use crate::check::{
    CheckComponentState, Constraint, PatternRelation, StaticTerm, TypeTerm, VariableId,
};

use super::Progress;
use super::queue::ConstraintQueue;

impl CheckComponentState<'_> {
    /// Solve collected component constraints to a fixed point.
    pub(in crate::check) fn solve(&mut self) -> CompilerResult<()> {
        let mut queue = ConstraintQueue::from(self.collect_constraints());

        loop {
            // step queued constraints until no variable changes
            while let Some(constraint) = queue.next() {
                let progress = self.step_constraint(&constraint)?;

                queue.wake(progress);
            }
            let progress = self.solve_bound_variables()?;
            if progress.is_unchanged() {
                break;
            }

            queue.wake_all();
        }

        Ok(())
    }

    /// Collect all component constraints in stable module order.
    pub(in crate::check) fn collect_constraints(&self) -> Vec<Constraint> {
        let mut constraints = Vec::new();

        // preserve module order and then local walk order
        for check_module in self.modules.values() {
            constraints.extend(check_module.work.constraints.iter().cloned());
        }

        constraints
    }

    /// Step one constraint once.
    fn step_constraint(&mut self, constraint: &Constraint) -> CompilerResult<Progress> {
        match constraint {
            Constraint::DefineType { result, term, .. } => self.step_type_definition(*result, term),
            Constraint::DefineStatic { result, term, .. } => {
                self.step_static_definition(*result, term)
            }
            Constraint::RelateType {
                relation,
                left,
                right,
                ..
            } => self.solve_type_relation(*relation, *left, *right),
            Constraint::RelateStatic {
                relation,
                left,
                right,
                ..
            } => self.solve_static_relation(*relation, *left, *right),
            Constraint::RelatePattern {
                relation, value, ..
            } => match relation {
                PatternRelation::Match(pattern) => self.propagate_pattern_relation(*value, pattern),
                PatternRelation::Assign(pattern) => {
                    self.propagate_assign_pattern_relation(*value, pattern)
                }
            },
        }
    }

    /// Step one type definition.
    fn step_type_definition(
        &mut self,
        result: VariableId,
        term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let reduction = self.reduce_type_term(result.module, term)?;
        let forward = match reduction.value {
            Some(term) => self.solve_type_variable(result, term)?,
            None => Progress::Unchanged,
        };
        let backward = self.propagate_type_expectation(result, term)?;

        Ok(reduction.progress.merge(forward).merge(backward))
    }

    /// Step one static definition constraint.
    fn step_static_definition(
        &mut self,
        result: VariableId,
        term: &StaticTerm,
    ) -> CompilerResult<Progress> {
        let Some(term) = self.reduce_static_term(result.module, term)? else {
            return Ok(Progress::Unchanged);
        };

        self.solve_static_variable(result, term)
    }
}
