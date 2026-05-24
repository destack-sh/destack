use crate::CompilerResult;
use crate::check::{CheckComponentState, Constraint};

use super::queue::{Progress, WorkQueue};

impl CheckComponentState<'_> {
    /// Solve collected component constraints to a fixed point.
    pub(in crate::check) fn solve(&mut self) -> CompilerResult<()> {
        let mut queue = WorkQueue::new(self.constraints());

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

    /// Return all component constraints in stable module order.
    pub(in crate::check) fn constraints(&self) -> Vec<Constraint> {
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
}
