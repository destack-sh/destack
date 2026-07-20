use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckEvent, CheckOutcome, CheckState, Constraint, ConstraintFailure, ConstraintId,
    Dependency, Origin, PlaceUse, Task, TaskFailure, TaskFailures, VariableRole,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Drain every queued task to quiescence.
    pub(in crate::check) fn drain(&mut self) -> CompilerResult<()> {
        loop {
            let task = self.solver.pop_task();
            let Some(task) = task else {
                break;
            };

            // move the task out of active work and report its failures
            match self.run_task(&task)? {
                Answer::Ready(failures) => {
                    self.solver.complete_task(&task);
                    for failure in failures {
                        self.report_task_failure(failure)?;
                    }
                }
                Answer::Pending(blockers) if blockers.is_empty() => {
                    return Err(CompilerError::Internal {
                        message: format!("check task {task:?} is pending without dependencies"),
                    });
                }
                Answer::Pending(blockers) => self.park_task(&task, &blockers)?,
            }

            self.record_event(CheckEvent::TaskRan {
                step: self.solve_steps,
                task,
            });
            self.solve_steps += 1;
        }

        Ok(())
    }

    /// Check named function bodies and settle every remaining task.
    pub(in crate::check) fn settle(&mut self) -> CompilerResult<()> {
        // queue every named function body
        for body in self.functions.values().copied().collect::<Vec<_>>() {
            self.queue_task(Task::CheckBody(body));
        }

        self.record_event(CheckEvent::SolveStarted {
            tasks: self.solver.queue.len(),
            variables: self.solver.variable_count(),
        });

        self.drain()?;
        self.close_memory_holes()?;
        self.report_unresolved()?;

        self.record_event(CheckEvent::SolveFinished {
            iterations: self.solve_steps,
            variables: self.solver.variable_count(),
        });

        Ok(())
    }

    /// Close un-evidenced lifetime holes to frame and access holes to readonly, then re-drain.
    fn close_memory_holes(&mut self) -> CompilerResult<()> {
        loop {
            let mut closed = false;
            for dependency in self.solver.waiting_dependencies() {
                let Dependency::Variable(variable) = dependency else {
                    continue;
                };
                let state = *self.solver.variable(variable)?;
                if state.solution.is_some() {
                    continue;
                }
                let default = match self.variable_memory_parameter(variable)? {
                    Some(dir::MemoryParameter::Lifetime) => {
                        dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame)
                    }
                    Some(dir::MemoryParameter::Access) => {
                        dir::MemoryLiteral::Access(dir::Access::Readonly)
                    }
                    _ => continue,
                };
                let origin = self.solver.origin(state.origin);
                let default = self.intern_type(origin.module(), dir::Type::Memory(default))?;
                self.commit_solution(variable, default)?;
                closed = true;
            }
            if !closed {
                return Ok(());
            }

            self.drain()?;
        }
    }

    /// Report every dependency still parked on after the drain.
    pub(in crate::check) fn report_unresolved(&mut self) -> CompilerResult<()> {
        // drain parked dependencies once
        let parked = self.solver.drain_waiters();
        let mut origins = FxIndexMap::default();

        // resolve each stuck dependency to the origin it anchors at
        for (dependency, _) in parked {
            let (origin, variable) = match dependency {
                Dependency::Variable(variable) => {
                    let state = self.solver.variable(variable)?;
                    if state.solution.is_some() {
                        continue;
                    }

                    (self.solver.origin(state.origin), Some(variable))
                }
                Dependency::SymbolType(symbol) => (Origin::Symbol(symbol), None),
            };
            origins.entry(origin).or_insert(variable);
        }

        self.report_cannot_infer_origins(origins)
    }

    /// Return the memory parameter kind one variable ranges over, however it arose.
    pub(in crate::check) fn variable_memory_parameter(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::MemoryParameter>> {
        let kind = match self.solver.variable_role(variable)? {
            VariableRole::Memory { kind, .. } => Some(kind),
            VariableRole::Instantiation { parameter } => self
                .generic_parameter(parameter)
                .and_then(|binding| binding.memory_parameter()),
            _ => None,
        };

        Ok(kind)
    }

    /// Report unresolved inference origins in deterministic source order.
    fn report_cannot_infer_origins(
        &mut self,
        origins: FxIndexMap<Origin, Option<dir::TypeVariableId>>,
    ) -> CompilerResult<()> {
        if origins.is_empty() {
            return Ok(());
        }

        let origins = origins.into_iter().collect::<Vec<_>>();

        // report in source order for deterministic diagnostics
        let mut keyed = Vec::new();
        for (origin, variable) in origins {
            let source = self.origin_source_node(origin)?;
            keyed.push((origin.module(), source.id, origin, variable));
        }
        keyed.sort_by_key(|(module, id, _, _)| (*module, *id));

        let mut reported = FxIndexSet::default();
        for (_, _, origin, variable) in keyed {
            self.report_cannot_infer_type(origin, variable, &mut reported)?;
        }

        Ok(())
    }

    /// Run one solver task once, returning its failed judgments.
    pub(in crate::check) fn run_task(
        &mut self,
        task: &Task,
    ) -> CompilerResult<Answer<TaskFailures>> {
        match task {
            Task::Relate(constraint) => self.run_relate(*constraint),
            Task::Check { site, expectation } => {
                let mut body = self.body();
                let checked = body.attempt_node(*site, PlaceUse::Read, Some(*expectation))?;

                Ok(match checked {
                    Answer::Ready(_) => Answer::Ready(TaskFailures::new()),
                    Answer::Pending(blockers) => Answer::Pending(blockers),
                })
            }
            Task::Infer { site, use_ } => {
                let mut body = self.body();
                let checked = body.attempt_node(*site, *use_, None)?;

                Ok(match checked {
                    Answer::Ready(_) => Answer::Ready(TaskFailures::new()),
                    Answer::Pending(blockers) => Answer::Pending(blockers),
                })
            }
            Task::CheckBody(body) => {
                let checked = body.check(self)?;

                Ok(match checked {
                    Answer::Ready(_) => Answer::Ready(TaskFailures::new()),
                    Answer::Pending(blockers) => Answer::Pending(blockers),
                })
            }
            Task::Oblige(obligation) => self.run_obligation(*obligation),
            Task::Solve { variable, mode } => self.solve_variable(*variable, *mode),
        }
    }

    /// Report one failed task judgment at the fulfillment boundary.
    fn report_task_failure(&mut self, failure: TaskFailure) -> CompilerResult<()> {
        match failure {
            TaskFailure::Constraint(failure) => self.report_constraint_failure(
                failure.cause,
                failure.relation,
                failure.use_,
                failure.source,
                failure.target,
                failure.failure,
            ),
            TaskFailure::Obligation(failure) => self.report_obligation_failure(failure),
        }
    }

    /// Solve one relation constraint once.
    fn run_relate(&mut self, id: ConstraintId) -> CompilerResult<Answer<TaskFailures>> {
        if self.solver.constraints.is_complete(id) {
            return Ok(Answer::Ready(TaskFailures::new()));
        }

        let constraint = *self.solver.constraints.get(id)?;
        let check = match &constraint {
            Constraint::Type(constraint) => self.check_type_constraint(
                constraint.cause,
                constraint.relation,
                constraint.source,
                constraint.target,
            )?,
            Constraint::Value(constraint) => {
                // park check-only relations until their target closes
                if constraint.is_check_only {
                    let target = self.settled_root(constraint.target)?;
                    let variables = self.type_variables(target)?;
                    if !variables.is_empty() {
                        let blockers: SmallVec<[Dependency; 2]> =
                            variables.into_iter().map(Dependency::Variable).collect();

                        return Ok(Answer::Pending(blockers));
                    }
                }

                self.check_value_constraint(
                    constraint.cause,
                    self.solver.origin(constraint.value_origin),
                    constraint.relation,
                    constraint.source,
                    constraint.target,
                )?
            }
        };

        match check {
            Answer::Ready(check) => {
                let mut failures = TaskFailures::new();
                if let CheckOutcome::Fails(failure) = check {
                    failures.push(TaskFailure::Constraint(ConstraintFailure {
                        cause: constraint.cause(),
                        relation: constraint.relation(),
                        use_: constraint.value_use(),
                        source: constraint.source(),
                        target: constraint.target(),
                        failure,
                    }));
                }

                let state = check.state();
                self.solver.set_constraint_state(id, state)?;
                self.record_event(CheckEvent::RelationChecked {
                    constraint: id,
                    is_finished: true,
                });

                Ok(Answer::Ready(failures))
            }
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

    /// Queue one solver task.
    pub(in crate::check) fn queue_task(&mut self, task: Task) {
        self.solver.push_task(task);
    }

    /// Park one task on its blocking dependencies.
    pub(in crate::check) fn park_task(
        &mut self,
        task: &Task,
        blockers: &[Dependency],
    ) -> CompilerResult<()> {
        self.record_event(CheckEvent::TaskParked {
            task: task.clone(),
            blockers: blockers.iter().copied().collect(),
        });

        for blocker in blockers {
            let dependency = match *blocker {
                Dependency::Variable(variable) => {
                    Dependency::Variable(self.solver.representative(variable)?)
                }
                Dependency::SymbolType(symbol) => Dependency::SymbolType(symbol),
            };
            self.solver.wait_for(dependency, task.clone());
        }

        Ok(())
    }
}
