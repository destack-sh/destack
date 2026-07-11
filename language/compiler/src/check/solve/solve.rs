use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::check::{
    Answer, BodyPhase, BodyState, CheckEvent, CheckOutcome, CheckState, Constraint,
    ConstraintFailure, ConstraintId, Dependency, Origin, Task, TaskFailure, TaskFailures,
};
use crate::{CompilerError, CompilerResult};

/// Which queued tasks one drain round runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TaskScope {
    /// Inference tasks only, leaving deferred obligations queued.
    Inference,
    /// Every queued task, including deferred obligations.
    All,
}

impl CheckState<'_> {
    /// Drain queued work within one task scope.
    pub(in crate::check) fn drain(&mut self, scope: TaskScope) -> CompilerResult<()> {
        loop {
            let task = match scope {
                TaskScope::All => self.solver.pop_task(),
                TaskScope::Inference => self.solver.pop_inference_task(),
            };
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

    /// Check every recorded body in source order, one phase at a time.
    pub(in crate::check) fn solve_bodies(&mut self) -> CompilerResult<()> {
        // run regular bodies in source order
        let mut index = 0;
        while index < self.bodies.len() {
            let owner = self.bodies[index];
            index += 1;
            if owner.phase != BodyPhase::Main {
                continue;
            }

            // skip bodies demanded out of order by symbol reads
            let already_bound = owner
                .binds
                .is_some_and(|(symbol, _)| self.symbol_type_maybe(symbol).is_some());
            if !already_bound {
                BodyState::run(self, owner)?;
            }
        }

        // handler bodies read failure unions completed by the bodies above
        let mut index = 0;
        while index < self.bodies.len() {
            let owner = self.bodies[index];
            index += 1;
            if owner.phase == BodyPhase::Handler {
                BodyState::run(self, owner)?;
            }
        }

        Ok(())
    }

    /// Run one symbol's recorded initializer body from a symbol read.
    pub(in crate::check) fn run_initializer(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let Some(index) = self.initializers.get(&symbol).copied() else {
            return Ok(());
        };

        // cut initializer cycles with an error binding
        if !self.initializing.insert(symbol) {
            let error = self.circular_type_error(Origin::Symbol(symbol))?;
            self.report(symbol.module_id, error);
            let poisoned = self.intern_type(symbol.module_id, dir::Type::Error)?;
            self.bind_symbol_type(symbol, poisoned)?;

            return Ok(());
        }
        let owner = self.bodies[index];
        BodyState::run(self, owner)?;
        self.initializing.swap_remove(&symbol);

        Ok(())
    }

    /// Settle every remaining obligation after all bodies checked.
    pub(in crate::check) fn settle(&mut self) -> CompilerResult<()> {
        self.record_event(CheckEvent::SolveStarted {
            tasks: self.solver.queue.len(),
            variables: self.solver.variable_count(),
        });

        // check function values no context ever received
        let lambdas = self.lambdas.values().copied().collect::<Vec<_>>();
        for owner in lambdas {
            if self.node_type_maybe_body(owner).is_none() {
                BodyState::run(self, owner)?;
            }
        }

        self.drain(TaskScope::All)?;
        self.report_parked_obligations()?;

        self.record_event(CheckEvent::SolveFinished {
            iterations: self.solve_steps,
            variables: self.solver.variable_count(),
        });

        Ok(())
    }

    /// Report every dependency still parked on after the drain.
    fn report_parked_obligations(&mut self) -> CompilerResult<()> {
        // drain parked dependencies once
        let parked = self.solver.drain_waiters();
        let mut origins = FxIndexSet::default();

        // resolve each stuck dependency to the origin it anchors at
        for (dependency, _) in parked {
            let origin = match dependency {
                Dependency::Variable(variable) => {
                    let state = self.solver.variable(variable)?;
                    if state.solution.is_some() {
                        continue;
                    }

                    self.solver.origin(state.origin)
                }
                Dependency::SymbolType(symbol) => Origin::Symbol(symbol),
            };
            origins.insert(origin);
        }

        self.report_cannot_infer_origins(origins)
    }

    /// Report unresolved inference origins in deterministic source order.
    fn report_cannot_infer_origins(&mut self, origins: FxIndexSet<Origin>) -> CompilerResult<()> {
        if origins.is_empty() {
            return Ok(());
        }

        let mut origins = origins.into_iter().collect::<Vec<_>>();

        // suppress casualties of errors reported before the sweep
        let mut tainted = FxIndexSet::default();
        for origin in &origins {
            let module = origin.module();
            if self.is_component_module(module) && !self.module(module).diagnostics.is_empty() {
                tainted.insert(module);
            }
        }
        origins.retain(|origin| !tainted.contains(&origin.module()));

        // report in source order for deterministic diagnostics
        let mut keyed = Vec::new();
        for origin in origins {
            let source = self.origin_source_node(origin)?;
            keyed.push((origin.module(), source.id, origin));
        }
        keyed.sort_by_key(|(module, id, _)| (*module, *id));

        let mut reported = FxIndexSet::default();
        for (_, _, origin) in keyed {
            self.report_cannot_infer_type(origin, &mut reported)?;
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
            Task::Oblige(obligation) => self.run_obligation(*obligation),
            Task::Solve { variable, mode } => self.solve_variable(*variable, *mode),
        }
    }

    /// Report one failed task judgment at the fulfillment boundary.
    fn report_task_failure(&mut self, failure: TaskFailure) -> CompilerResult<()> {
        match failure {
            TaskFailure::Constraint(failure) => self.report_constraint_failure(
                failure.origin,
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
                self.solver.origin(constraint.origin),
                constraint.relation,
                constraint.subject,
                constraint.source,
                constraint.target,
            )?,
            Constraint::Value(constraint) => self.check_value_constraint(
                self.solver.origin(constraint.origin),
                self.solver.origin(constraint.value_origin),
                constraint.relation,
                constraint.source,
                constraint.target,
            )?,
        };

        match check {
            Answer::Ready(check) => {
                let mut failures = TaskFailures::new();
                if let CheckOutcome::Fails(failure) = check {
                    failures.push(TaskFailure::Constraint(ConstraintFailure {
                        origin: self.solver.origin(constraint.origin()),
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
