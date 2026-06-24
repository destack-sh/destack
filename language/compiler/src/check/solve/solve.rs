use smallvec::SmallVec;

use crate::check::{
    Answer, CheckEvent, CheckState, Condition, Constraint, ConstraintId, Dependency, Mutation,
    Task, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Solve collected component constraints to a fixed point.
    pub(in crate::check) fn solve(&mut self) -> CompilerResult<()> {
        // the journal records only while a probe is active
        if self.journal.is_active() {
            return Err(CompilerError::Internal {
                message: "solver cannot drain work while a probe is active".into(),
            });
        }

        self.record_event(CheckEvent::SolveStarted {
            tasks: self.queue.len(),
            variables: self.variables.count(),
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
            variables: self.variables.count(),
        });

        Ok(())
    }

    /// Run one solver task once.
    fn run_task(&mut self, task: Task) -> CompilerResult<Answer<()>> {
        match task {
            Task::Relate(constraint) => self.run_relate(constraint),
            Task::Decide(node) => self.run_decide(node),
            Task::Solve(variable) => self.run_solve(variable),
        }
    }

    /// Solve one relation constraint once.
    fn run_relate(&mut self, id: ConstraintId) -> CompilerResult<Answer<()>> {
        if self.constraints.is_complete(id) {
            return Ok(Answer::Ready(()));
        }

        // copy the relation before solver calls can mutate state
        let (relation, cause, left, right, origin, coercion_site, predicates) = {
            let constraint = self.constraints.get(id)?;
            let predicates = match &constraint.condition {
                Condition::Always => SmallVec::new(),
                Condition::When(predicates) => predicates.clone(),
            };

            (
                constraint.relation,
                constraint.cause,
                constraint.left,
                constraint.right,
                constraint.origin,
                constraint.coercion_site,
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
            self.complete_constraint(id);

            return Ok(Answer::Ready(()));
        }

        // solve active relations under their guard assumptions
        let mark = self.assume(&predicates)?;
        let answer = self.relate(origin, relation, cause, left, right);
        self.release_assumptions(mark);

        match answer? {
            Answer::Ready(()) => {
                answer!(self.insert_implicit_coercion(
                    origin,
                    relation,
                    left,
                    right,
                    coercion_site,
                    &predicates,
                )?);

                self.complete_constraint(id);
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

    /// Collect one journaled constraint and schedule it.
    pub(in crate::check) fn push_constraint(&mut self, constraint: Constraint) -> ConstraintId {
        let id = self.constraints.allocate(constraint);
        self.journal.record(Mutation::ConstraintAllocated);
        self.queue_task(Task::Relate(id));

        id
    }

    /// Queue one solver task.
    pub(in crate::check) fn queue_task(&mut self, task: Task) {
        self.queue.push(task);
        self.journal.record(Mutation::TaskQueued { task });
    }

    /// Pop the next solver task.
    fn pop_task(&mut self) -> Option<Task> {
        let task = self.queue.pop();

        if let Some(task) = task {
            self.journal.record(Mutation::TaskPopped { task });
        }

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
                    let representative = self.variables.representative(variable)?;
                    let state = self.variables.get_mut(representative)?;

                    if !state.waiters.contains(&task) {
                        state.waiters.push(task);
                        self.journal.record(Mutation::WaiterPushed {
                            variable: representative,
                        });
                    }
                }
                // park on the undecided node
                Dependency::Decision(node) => {
                    self.decisions.wait(node, task);
                    self.journal.record(Mutation::DecisionWaiterPushed { node });
                }
            }
        }

        Ok(())
    }

    /// Mark one constraint complete.
    pub(in crate::check) fn complete_constraint(&mut self, id: ConstraintId) {
        self.constraints.complete(id);
        self.journal
            .record(Mutation::ConstraintCompleted { constraint: id });
    }
}
