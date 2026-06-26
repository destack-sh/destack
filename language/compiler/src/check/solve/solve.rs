use smallvec::SmallVec;

use crate::check::{
    Answer, CheckEvent, CheckState, Condition, Constraint, ConstraintId, ConstraintState,
    Dependency, Task, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Solve collected component constraints to a fixed point.
    pub(in crate::check) fn solve(&mut self) -> CompilerResult<()> {
        self.record_event(CheckEvent::SolveStarted {
            tasks: self.solver.queue.len(),
            variables: self.solver.variable_count(),
        });

        let mut steps = 0usize;
        while let Some(task) = self.pop_task() {
            let answer = self.run_task(task)?;

            // park pending tasks on their blockers
            if let Answer::Pending(blockers) = answer {
                self.park_task(task, &blockers)?;
            }

            self.record_event(CheckEvent::TaskRan { step: steps, task });
            steps += 1;
        }

        self.record_event(CheckEvent::SolveFinished {
            iterations: steps,
            variables: self.solver.variable_count(),
        });

        Ok(())
    }

    /// Run one solver task once.
    fn run_task(&mut self, task: Task) -> CompilerResult<Answer<()>> {
        match task {
            Task::Relate(constraint) => self.run_relate(constraint),
            Task::Select(selection) => self.run_select(selection),
            Task::Oblige(obligation) => self.run_obligation(obligation),
            Task::Solve(variable) => self.run_solve(variable),
        }
    }

    /// Solve one relation constraint once.
    fn run_relate(&mut self, id: ConstraintId) -> CompilerResult<Answer<()>> {
        if self.solver.constraints.is_complete(id) {
            return Ok(Answer::Ready(()));
        }

        // copy the relation before solver calls can mutate state
        let (relation, value_use, left, right, origin, predicates) = {
            let constraint = self.solver.constraints.get(id)?;
            let predicates = match constraint.condition() {
                Condition::Always => SmallVec::new(),
                Condition::When(predicates) => predicates.clone(),
            };

            (
                constraint.relation(),
                constraint.value_use(),
                constraint.left(),
                constraint.right(),
                constraint.origin(),
                predicates,
            )
        };

        // skip conditional constraints whose predicates failed
        let is_active = if predicates.is_empty() {
            true
        } else {
            answer!(self.decide_condition(&predicates)?)
        };
        if !is_active {
            self.set_constraint_state(id, ConstraintState::Skipped)?;

            return Ok(Answer::Ready(()));
        }

        // solve active relations under their guard assumptions
        let mark = self.assume(&predicates)?;
        let answer = self.constrain(origin, relation, left, right);
        self.release_assumptions(mark);

        match answer? {
            Answer::Ready(holds) => {
                if !holds {
                    self.report_relation_failure(origin, relation, value_use, left, right)?;
                }

                let state = if holds {
                    ConstraintState::Holds
                } else {
                    ConstraintState::Fails
                };
                self.set_constraint_state(id, state)?;
                self.record_event(CheckEvent::RelationChecked {
                    constraint: id,
                    is_finished: true,
                });

                Ok(Answer::Ready(()))
            }
            Answer::Pending(blockers) if blockers.is_empty() => Err(CompilerError::Internal {
                message: format!("check constraint {id:?} is pending without dependencies"),
            }),
            Answer::Pending(blockers) => {
                self.record_event(CheckEvent::RelationChecked {
                    constraint: id,
                    is_finished: false,
                });

                Ok(Answer::Pending(blockers))
            }
        }
    }

    /// Collect one constraint and schedule it.
    pub(in crate::check) fn push_constraint(&mut self, constraint: Constraint) -> ConstraintId {
        let id = self.solver.allocate_constraint(constraint);
        self.queue_task(Task::Relate(id));

        id
    }

    /// Collect one selection and schedule it.
    pub(in crate::check) fn push_selection(&mut self, selection: crate::check::Selection) {
        let id = self.solver.allocate_selection(selection);
        self.queue_task(Task::Select(id));
    }

    /// Queue one solver task.
    pub(in crate::check) fn queue_task(&mut self, task: Task) {
        self.solver.push_task(task);
    }

    /// Pop the next solver task.
    fn pop_task(&mut self) -> Option<Task> {
        let task = self.solver.pop_task();

        task
    }

    /// Park one task on its blocking dependencies.
    pub(in crate::check) fn park_task(
        &mut self,
        task: Task,
        blockers: &[Dependency],
    ) -> CompilerResult<()> {
        for blocker in blockers {
            match *blocker {
                // park on the variable representative
                Dependency::Variable(variable) => {
                    let representative = self.solver.representative(variable)?;
                    let state = self.solver.variable_mut(representative)?;

                    if !state.waiters.contains(&task) {
                        state.waiters.push(task);
                    }
                }
                // park on the undecided node
                Dependency::Decision(node) => {
                    self.solver.wait_for_decision(node, task);
                }
            }
        }

        Ok(())
    }

    /// Set one constraint state.
    pub(in crate::check) fn set_constraint_state(
        &mut self,
        id: ConstraintId,
        state: ConstraintState,
    ) -> CompilerResult<()> {
        self.solver.set_constraint_state(id, state)
    }
}
