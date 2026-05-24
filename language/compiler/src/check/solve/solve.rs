use crate::CompilerResult;
use crate::check::{CheckComponentState, Constraint, StaticTerm, TypeTerm, VariableId};

use super::queue::{ConstraintQueue, Progress};

impl CheckComponentState<'_> {
    /// Solve collected component constraints to a fixed point.
    pub(in crate::check) fn solve(&mut self) -> CompilerResult<()> {
        let mut queue = ConstraintQueue::new(self.collect_constraints());

        loop {
            // reduce queued constraints until no variable changes
            while let Some(constraint) = queue.next() {
                let progress = self.reduce_constraint(&constraint)?;

                queue.wake(progress);
            }
            let progress = self.solve_bound_variables()?;
            if progress.is_unchanged() {
                break;
            }

            queue.wake(progress);
        }

        Ok(())
    }

    /// Collect all component constraints in stable module order.
    pub(in crate::check) fn collect_constraints(&self) -> Vec<Constraint> {
        let mut constraints = Vec::new();

        // preserve module order and then local walk order
        for check_module in self.modules.values() {
            constraints.extend(check_module.constraints().iter().cloned());
        }

        constraints
    }

    /// Reduce one constraint once.
    fn reduce_constraint(&mut self, constraint: &Constraint) -> CompilerResult<Progress> {
        match constraint {
            Constraint::DefineType { result, term, .. } => {
                self.reduce_type_definition(*result, term)
            }
            Constraint::DefineStatic { result, term, .. } => {
                self.reduce_static_definition(*result, term)
            }
            Constraint::RelateType {
                relation,
                left,
                right,
                ..
            } => self.relate_type(*relation, *left, *right),
            Constraint::RelateStatic {
                relation,
                left,
                right,
                ..
            } => self.relate_static(*relation, *left, *right),
            Constraint::RequirePlaceWrite { .. } => Ok(Progress::Unchanged),
        }
    }

    /// Reduce one type definition.
    fn reduce_type_definition(
        &mut self,
        result: VariableId,
        term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let forward = match self.reduce_type_term(result.module, term)? {
            Some(term) => self.solve_type_variable(result, term)?,
            None => Progress::Unchanged,
        };
        let backward = self.expect_type_term(result, term)?;

        Ok(forward.merge(backward))
    }

    /// Reduce one static definition constraint.
    fn reduce_static_definition(
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
