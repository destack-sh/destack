use crate::CompilerResult;
use crate::check::{
    CheckState, Constraint, Decision, Definition, Origin, PatternRelation, StaticTerm,
    TypeOperationTerm, TypeTerm, VariableId,
};

use super::Progress;
use super::solver::{SolveTask, Solver};

impl CheckState<'_> {
    /// Solve collected component constraints to a fixed point.
    pub(in crate::check) fn solve(&mut self) -> CompilerResult<()> {
        let tasks = self.initial_solve_tasks();
        let mut solver = Solver::new(tasks, self);

        // step queued tasks until no variable changes
        while let Some((index, task)) = solver.next() {
            let progress = self.step_solve_task(task)?;
            solver.discover_tasks(self);
            solver.refresh(index, self);
            solver.wake(progress, self);
        }

        Ok(())
    }

    /// Collect all component solver tasks in stable module order.
    fn initial_solve_tasks(&self) -> Vec<SolveTask> {
        let mut tasks = Vec::new();

        let variables = &self.variables;

        tasks.extend(
            variables
                .definitions
                .iter()
                .enumerate()
                .map(|(index, _)| SolveTask::Definition(index)),
        );
        tasks.extend(
            variables
                .constraints
                .iter()
                .enumerate()
                .map(|(index, _)| SolveTask::Constraint(index)),
        );

        tasks
    }

    /// Step one solver task once.
    fn step_solve_task(&mut self, task: SolveTask) -> CompilerResult<Progress> {
        match task {
            SolveTask::Definition(index) => self.step_definition(index),
            SolveTask::Constraint(index) => self.step_constraint(index),
            SolveTask::Variable(variable) => self.solve_bound_variable(variable),
        }
    }

    /// Step one definition once.
    fn step_definition(&mut self, index: usize) -> CompilerResult<Progress> {
        let condition = match &self.variables.definitions[index] {
            Definition::Type { condition, .. } | Definition::Static { condition, .. } => condition,
        }
        .clone();

        if self.reduce_condition_decision(&condition)? == Decision::No {
            return Ok(Progress::Unchanged);
        }

        match &self.variables.definitions[index] {
            Definition::Type {
                result,
                term,
                origin,
                condition: _,
            } => {
                let origin = *origin;
                let result = *result;
                let term = self.terms.get(*term).clone();

                self.step_type_definition(origin, result, &term)
            }
            Definition::Static {
                result,
                term,
                origin,
                condition: _,
            } => {
                let origin = *origin;
                let result = *result;
                let term = self.terms.get(*term).clone();

                self.step_static_definition(origin, result, &term)
            }
        }
    }

    /// Step one constraint once.
    fn step_constraint(&mut self, index: usize) -> CompilerResult<Progress> {
        let condition = self.variables.constraints[index].condition();
        if self.reduce_condition_decision(&condition)? != Decision::Yes {
            return Ok(Progress::Unchanged);
        }

        match &self.variables.constraints[index] {
            Constraint::Type {
                relation,
                left,
                right,
                origin,
                condition: _,
            } => {
                let origin = *origin;
                let relation = *relation;
                let left = *left;
                let right = *right;

                self.solve_type_relation(origin, relation, left, right)
            }
            Constraint::Pattern {
                relation,
                value,
                origin,
                condition: _,
            } => match relation {
                PatternRelation::Match(pattern) => {
                    self.expect_pattern_term(*origin, *value, *pattern)
                }
                PatternRelation::Assign(pattern) => {
                    self.expect_assign_pattern_term(*origin, *value, *pattern)
                }
            },
        }
    }

    /// Step one type definition.
    fn step_type_definition(
        &mut self,
        origin: Origin,
        result: VariableId,
        term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let reduction = self.reduce_type_term(origin, term)?;
        let forward = match reduction.value {
            Some(term) => self.solve_type_variable(result, term)?,
            None if self.type_term_is_durable(term) => {
                self.solve_type_variable(result, term.clone())?
            }
            None => Progress::Unchanged,
        };
        let backward = self.expect_type_term(origin, result, term)?;

        Ok(reduction.progress.merge(forward).merge(backward))
    }

    /// Return whether an unreduced type term can be committed as type structure.
    fn type_term_is_durable(&self, term: &TypeTerm) -> bool {
        let TypeTerm::Operation(operation) = term else {
            return false;
        };

        matches!(
            self.terms.get(*operation),
            TypeOperationTerm::Conditional { .. }
                | TypeOperationTerm::Index { .. }
                | TypeOperationTerm::TemplateLiteral { .. }
                | TypeOperationTerm::Infer { .. }
                | TypeOperationTerm::KeyOf { .. }
                | TypeOperationTerm::Mapped { .. }
        )
    }

    /// Step one static definition.
    fn step_static_definition(
        &mut self,
        origin: Origin,
        result: VariableId,
        term: &StaticTerm,
    ) -> CompilerResult<Progress> {
        if let Some(term) = self.reduce_static_term(origin, term)? {
            return self.solve_static_variable(result, term);
        }

        match term {
            // preserve parametric static source for generic substitution
            StaticTerm::Expression(_) | StaticTerm::Variable(_) | StaticTerm::Parameter(_) => {
                self.solve_static_variable(result, term.clone())
            }
            _ => Ok(Progress::Unchanged),
        }
    }
}
