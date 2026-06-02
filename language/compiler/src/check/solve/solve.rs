use crate::CompilerResult;
use crate::check::{
    CheckEvent, CheckState, Constraint, Decision, PatternRelation, SolveProgress, StaticRelation,
};

use super::Progress;

impl CheckState<'_> {
    /// Solve collected component constraints to a fixed point.
    pub(in crate::check) fn solve(&mut self) -> CompilerResult<()> {
        self.record_trace(CheckEvent::SolveStart {
            tasks: self.inference.constraint_count() + self.variable_count(),
            variables: self.variable_count(),
        });

        let mut iterations = 0;
        loop {
            let constraints_before = self.inference.constraint_count();
            let variables_before = self.variable_count();
            let progress = self.step_solve_pass()?;
            let progress_summary = SolveProgress::from(&progress);

            self.record_trace(CheckEvent::SolveStep {
                step: iterations,
                progress: progress_summary,
            });
            iterations += 1;

            let counts_changed = self.inference.constraint_count() != constraints_before
                || self.variable_count() != variables_before;
            if progress.is_unchanged() && !counts_changed {
                break;
            }
        }

        self.record_trace(CheckEvent::SolveFinish {
            iterations,
            variables: self.variable_count(),
        });

        Ok(())
    }

    /// Step all collected solver work once.
    fn step_solve_pass(&mut self) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        for index in 0..self.inference.constraint_count() {
            progress = progress.merge(self.step_constraint(index)?);
        }

        let mut index = 0;
        while index < self.variable_count() {
            let variable = self.variable_at(index).id;

            progress = progress.merge(self.solve_bound_variable(variable)?);
            index += 1;
        }

        Ok(progress)
    }

    /// Step one constraint once.
    fn step_constraint(&mut self, index: usize) -> CompilerResult<Progress> {
        let constraint = self.inference.constraint(index).clone();
        let condition = constraint.condition();
        if self.reduce_condition_decision(&condition)? != Decision::Yes {
            return Ok(Progress::Unchanged);
        }

        match constraint {
            Constraint::Type {
                relation,
                left,
                right,
                origin,
                condition: _,
            } => self.relate_type_relation(origin, relation, left, right),
            Constraint::Static {
                relation,
                left,
                right,
                origin: _,
                condition: _,
            } => match relation {
                StaticRelation::Equal => self.relate_static_equality(left, right),
                StaticRelation::Assignable => self.relate_static_assignability(left, right),
            },
            Constraint::Pattern {
                relation,
                value,
                origin,
                condition: _,
            } => match relation {
                PatternRelation::Match(pattern) => self.expect_pattern_term(origin, value, pattern),
                PatternRelation::Assign(pattern) => {
                    self.expect_assign_pattern_term(origin, value, pattern)
                }
            },
        }
    }
}
