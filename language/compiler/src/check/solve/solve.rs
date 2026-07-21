use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;

use crate::check::{
    Answer, CheckEvent, CheckOutcome, CheckState, Constraint, ConstraintFailure, ConstraintId,
    ConstraintState, Dependency, Expectation, Origin, Task, TaskFailure, TaskFailures, ValueSource,
    VariableDomain, VariableRole, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Drain every queued task to quiescence.
    pub(in crate::check) fn drain(&mut self) -> CompilerResult<()> {
        let failures = self.drain_tasks()?;
        for failure in failures {
            self.report_task_failure(failure)?;
        }

        Ok(())
    }

    /// Drain every queued task and return its failed judgments.
    pub(in crate::check) fn drain_tasks(&mut self) -> CompilerResult<TaskFailures> {
        let mut failures = TaskFailures::new();
        loop {
            // run every ready value and type judgment before solving
            if let Some(task) = self.solver.pop_judgment() {
                self.run_queued_task(task, &mut failures)?;

                continue;
            }

            // solve grounded inference before checking deferred obligations
            if self.solve_variables(VariableDomain::ALL)? {
                continue;
            }

            // obligations may add new judgments, so check one at a time
            if let Some(task) = self.solver.pop_obligation() {
                self.run_queued_task(task, &mut failures)?;

                continue;
            }

            // apply defaults only after every available obligation ran
            if self.default_variables(VariableDomain::ALL)? {
                continue;
            }

            break;
        }

        Ok(failures)
    }

    /// Drain queued value and type constraints without checking bodies or obligations.
    pub(in crate::check) fn drain_constraint_tasks(
        &mut self,
        variables: VariableDomain,
    ) -> CompilerResult<TaskFailures> {
        let mut failures = TaskFailures::new();
        loop {
            // run only value and type constraints created by the probe
            if let Some(task) = self.solver.pop_constraint_task() {
                self.run_queued_task(task, &mut failures)?;

                continue;
            }

            // solve variables allocated by the probe from its accepted constraints
            if self.solve_variables(variables)? {
                continue;
            }

            // apply probe-local defaults after all available evidence
            if self.default_variables(variables)? {
                continue;
            }

            break;
        }

        Ok(failures)
    }

    /// Run one popped task, completing or parking it.
    fn run_queued_task(&mut self, task: Task, failures: &mut TaskFailures) -> CompilerResult<()> {
        // move the task out of active work and collect its failures
        match self.run_task(&task)? {
            Answer::Ready(task_failures) => {
                self.solver.complete_task(&task);
                failures.extend(task_failures);
            }
            Answer::Pending(blockers) => {
                if blockers.is_empty() {
                    return Err(CompilerError::Internal {
                        message: format!("check task {task:?} is pending without dependencies"),
                    });
                }

                self.park_task(&task, &blockers)?;
            }
        }

        self.record_event(CheckEvent::TaskRan {
            step: self.solve_steps,
            task,
        });
        self.solve_steps += 1;

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
        self.report_unresolved()?;

        self.record_event(CheckEvent::SolveFinished {
            iterations: self.solve_steps,
            variables: self.solver.variable_count(),
        });

        Ok(())
    }

    /// Report every unresolved symbol and inference variable after the drain.
    pub(in crate::check) fn report_unresolved(&mut self) -> CompilerResult<()> {
        // retain unresolved symbol dependencies from parked tasks
        let parked = self.solver.drain_waiters();
        let mut origins = FxIndexMap::default();
        for (dependency, _) in parked {
            if let Dependency::SymbolType(symbol) = dependency {
                origins.entry(Origin::Symbol(symbol)).or_insert(None);
            }
        }

        // every remaining root is a genuine inference failure
        let mut unresolved = Vec::new();
        for index in 0..self.solver.variable_count() {
            let variable = dir::TypeVariableId(index as u32);
            let state = *self.solver.variable(variable)?;
            if state.solution.is_none() {
                let origin = self.solver.origin(state.origin);
                origins.entry(origin).or_insert(Some(variable));
                unresolved.push((variable, origin.module()));
            }
        }

        self.report_cannot_infer_origins(origins)?;

        // close failed inference graphs with the compiler error type
        for (variable, module) in unresolved {
            if self.solver.variable(variable)?.solution.is_some() {
                continue;
            }
            let error = self.intern_type(module, dir::Type::Error)?;
            self.commit_solution(variable, error)?;
        }

        Ok(())
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
            Task::Relate(constraint) | Task::Check(constraint) => self.run_relate(*constraint),
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
        let checked = match constraint {
            Constraint::Type(constraint) => {
                let check = self.check_type_constraint(
                    constraint.cause,
                    constraint.relation,
                    constraint.source,
                    constraint.target,
                )?;

                match check {
                    Answer::Ready(check) => {
                        Answer::Ready((constraint.source, constraint.target, check, None))
                    }
                    Answer::Pending(blockers) => Answer::Pending(blockers),
                }
            }
            Constraint::Value(constraint) => {
                let mut body = self.body();
                let target = constraint.target;
                match constraint.source {
                    ValueSource::Node(node) => {
                        let site = body.node_site(node)?;
                        let expectation = Expectation {
                            target,
                            relation: constraint.relation,
                            cause: constraint.cause,
                            use_: constraint.use_,
                        };
                        let check = answer!(body.check_node_target(site, expectation)?);
                        let source = answer!(body.node_type_at(site)?);

                        Answer::Ready((source, target, check.outcome, Some(check.target)))
                    }
                    ValueSource::Type(source) => {
                        let check = body.check_value_relation(
                            constraint.cause,
                            constraint.relation,
                            source,
                            target,
                        )?;

                        match check {
                            Answer::Ready(check) => {
                                Answer::Ready((source, target, check.outcome, Some(check.target)))
                            }
                            Answer::Pending(blockers) => Answer::Pending(blockers),
                        }
                    }
                }
            }
        };

        match checked {
            Answer::Ready((source, target, check, value_target)) => {
                let mut failures = TaskFailures::new();
                if let CheckOutcome::Fails(failure) = check {
                    failures.push(TaskFailure::Constraint(ConstraintFailure {
                        cause: constraint.cause(),
                        relation: constraint.relation(),
                        use_: constraint.value_use(),
                        source,
                        target,
                        failure,
                    }));
                }

                let state = check.state();
                self.solver.set_constraint_result(id, state, value_target)?;
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
        let task = match constraint {
            Constraint::Value(constraint) if matches!(constraint.source, ValueSource::Node(_)) => {
                Task::Check(id)
            }
            Constraint::Type(_) | Constraint::Value(_) => Task::Relate(id),
        };
        self.queue_task(task);

        id
    }

    /// Record one constraint that has already finished checking.
    pub(in crate::check) fn record_constraint(
        &mut self,
        constraint: Constraint,
        state: ConstraintState,
        value_target: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<ConstraintId> {
        if !state.is_done() {
            return Err(CompilerError::Internal {
                message: "recorded constraint is still pending".to_string(),
            });
        }

        let id = self.solver.allocate_constraint(constraint);
        self.solver.set_constraint_result(id, state, value_target)?;

        Ok(id)
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
            self.solver.wait_for(*blocker, task.clone());
        }

        Ok(())
    }
}
