use crate::CompilerResult;
use crate::check::{
    CheckEvent, CheckState, Constraint, ConstraintId, Decision, SolveTask, VariableId, VariableKind,
};

impl CheckState<'_> {
    /// Solve collected component constraints to a fixed point.
    pub(in crate::check) fn solve(&mut self) -> CompilerResult<()> {
        self.record_event(CheckEvent::SolveStart {
            tasks: self.inference.solve_task_count(),
            variables: self.variable_count(),
        });

        let mut steps = 0;
        while let Some(task) = self.inference.pop_solve_task() {
            match task {
                SolveTask::Constraint(constraint) => self.reduce_constraint(constraint)?,
                SolveTask::Variable(variable) => self.reduce_variable(variable)?,
            }

            self.record_event(CheckEvent::SolveStep { step: steps });
            steps += 1;
        }

        self.record_event(CheckEvent::SolveFinish {
            iterations: steps,
            variables: self.variable_count(),
        });

        Ok(())
    }

    /// Reduce one active constraint once.
    fn reduce_constraint(&mut self, id: ConstraintId) -> CompilerResult<()> {
        let constraint = self.inference.constraint_by_id(id).clone();
        let condition = constraint.condition();
        if self.reduce_condition_decision(&condition)? != Decision::Yes {
            return Ok(());
        }

        match constraint {
            Constraint::Type {
                relation,
                left,
                right,
                origin,
                condition: _,
            } => self.reduce_type_relation(origin, relation, left, right)?,
            Constraint::Static {
                relation,
                left,
                right,
                origin: _,
                condition: _,
            } => self.reduce_static_relation(relation, left, right)?,
            Constraint::Pattern {
                relation,
                value,
                origin,
                condition: _,
            } => self.reduce_pattern_relation(origin, relation, value)?,
        };

        Ok(())
    }

    /// Solve one variable from its collected bounds.
    fn reduce_variable(&mut self, variable: VariableId) -> CompilerResult<()> {
        // skip solved variables
        if self.variable_solution(variable).is_some() {
            return Ok(());
        }

        // dispatch by variable domain
        let kind = self.variable(variable).kind;
        match kind {
            VariableKind::Type => self.solve_type_variable_from_bounds(variable),
            VariableKind::Static => self.solve_static_variable_from_bounds(variable),
        }
    }
}
