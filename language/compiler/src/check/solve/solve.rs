use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;

use crate::check::{
    Answer, BoundSide, CheckEvent, CheckFailure, CheckOutcome, CheckState, Constraint,
    ConstraintFailure, ConstraintId, ConstraintState, Dependency, Expectation, FlowSite,
    InferenceScope, Origin, Task, TaskFailure, TaskFailures, VariableRole, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Drain every queued task to quiescence.
    pub(in crate::check) fn drain(&mut self) -> CompilerResult<()> {
        let failures = self.drain_tasks()?;

        // report the most specific failed constraint in each cause chain
        let mut report = Vec::with_capacity(failures.len());
        for (index, failure) in failures.iter().enumerate() {
            let TaskFailure::Constraint(failure) = failure else {
                report.push(index);

                continue;
            };
            let has_descendant = failures.iter().any(|candidate| {
                let TaskFailure::Constraint(candidate) = candidate else {
                    return false;
                };

                self.solver
                    .causes
                    .is_ancestor(failure.cause, candidate.cause)
            });
            if !has_descendant {
                report.push(index);
            }
        }

        for (index, failure) in failures.into_iter().enumerate() {
            if !report.contains(&index) {
                continue;
            }
            self.report_task_failure(failure)?;
        }

        Ok(())
    }

    /// Drain every queued task and return its failures.
    pub(in crate::check) fn drain_tasks(&mut self) -> CompilerResult<TaskFailures> {
        let mut failures = TaskFailures::new();
        loop {
            // run every ready check task first
            if let Some(task) = self.solver.pop_check() {
                self.run_queued_task(task, &mut failures)?;

                continue;
            }

            // settle component-owned variables before obligations consume them
            let mark = self.solver.snapshot_watermark();
            if self.settle_task(InferenceScope::ROOT, mark, false)?.settled {
                continue;
            }

            // obligations may add new check tasks, so run one at a time
            if let Some(task) = self.solver.pop_obligation() {
                self.run_queued_task(task, &mut failures)?;

                continue;
            }

            // defaults complete dry variables only at quiescence
            let mark = self.solver.snapshot_watermark();
            if self.settle_task(InferenceScope::ROOT, mark, true)?.settled {
                continue;
            }

            break;
        }

        Ok(failures)
    }

    /// Drain queued value and type constraints without checking bodies or obligations.
    pub(in crate::check) fn drain_constraint_tasks(
        &mut self,
        scope: InferenceScope,
        mark: usize,
    ) -> CompilerResult<TaskFailures> {
        let mut failures = TaskFailures::new();
        loop {
            // run every value and type constraint created by the task
            while let Some(task) = self.solver.pop_constraint_task() {
                self.run_queued_task(task, &mut failures)?;
            }

            // settle the variables the task owns and the holes it adopted
            if !self.settle_task(scope, mark, true)?.settled {
                break;
            }
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
        // every remaining root is a genuine inference failure
        let mut unresolved = Vec::new();
        for index in 0..self.solver.variable_count() {
            let variable = dir::TypeVariableId(index as u32);
            let state = *self.solver.variable(variable)?;
            if state.solution.is_none() {
                let origin = self.solver.origin(state.origin);
                unresolved.push((variable, origin.module()));
            }
        }

        // one connected open inference graph is one failure: it reports at
        //  its annotatable produced cells, or at every origin without one
        let mut origins = FxIndexMap::default();
        for group in self.unresolved_variable_groups()? {
            let annotatable = group
                .iter()
                .filter(|variable| {
                    matches!(
                        self.solver.variable_role(**variable),
                        Ok(VariableRole::Return)
                    )
                })
                .copied()
                .collect::<Vec<_>>();
            let representatives = match annotatable.is_empty() {
                true => &group,
                false => &annotatable,
            };
            for variable in representatives {
                let state = *self.solver.variable(*variable)?;
                let origin = self.solver.origin(state.origin);
                origins.entry(origin).or_insert(Some(*variable));
            }
        }

        // report the failures while their bounds still show as open
        self.report_cannot_infer_origins(origins)?;

        // close failed inference graphs with the compiler error type;
        //  leave declarations open for their inference components
        if !self.is_declaration() {
            let poisoned = !unresolved.is_empty();
            for (variable, module) in unresolved {
                if self.solver.variable(variable)?.solution.is_some() {
                    continue;
                }
                let error = self.intern_type(module, dir::Type::Error)?;
                self.commit_solution(variable, error)?;
            }

            // poisoned graphs complete their parked tasks against the error
            if poisoned {
                self.drain()?;
            }
        }

        // retain unresolved symbol dependencies from parked tasks
        let parked = self.solver.drain_waiters();
        let mut symbols = FxIndexMap::default();
        for (dependency, _) in parked {
            if let Dependency::SymbolType(symbol) = dependency {
                symbols.entry(Origin::Symbol(symbol)).or_insert(None);
            }
        }
        self.report_cannot_infer_origins(symbols)?;

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

        // keep inferred members' origins; interface members report in their own component
        let origins = origins
            .into_iter()
            .filter(|(origin, _)| self.infers_module(origin.module()))
            .collect::<Vec<_>>();

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

    /// Group open variables into weakly connected inference graphs.
    fn unresolved_variable_groups(&self) -> CompilerResult<Vec<Vec<dir::TypeVariableId>>> {
        let count = self.solver.variable_count();
        let mut parent: Vec<u32> = (0..count as u32).collect();

        fn find(parent: &mut [u32], mut index: u32) -> u32 {
            while parent[index as usize] != index {
                parent[index as usize] = parent[parent[index as usize] as usize];
                index = parent[index as usize];
            }

            index
        }

        // union open variables with the open variables in their bounds
        for index in 0..count {
            let variable = dir::TypeVariableId(index as u32);
            if self.solver.variable(variable)?.solution.is_some() {
                continue;
            }
            for side in [BoundSide::Lower, BoundSide::Upper] {
                for bound in self.solver.variables.side_bounds(variable, side)? {
                    for dependency in self.type_variables(bound.ty)? {
                        if self.solver.variable(dependency)?.solution.is_none() {
                            let left = find(&mut parent, index as u32);
                            let right = find(&mut parent, dependency.0);
                            parent[left as usize] = right;
                        }
                    }
                }
            }
        }

        // collect groups in first-member order
        let mut groups = FxIndexMap::<u32, Vec<dir::TypeVariableId>>::default();
        for index in 0..count {
            let variable = dir::TypeVariableId(index as u32);
            if self.solver.variable(variable)?.solution.is_none() {
                let root = find(&mut parent, index as u32);
                groups.entry(root).or_default().push(variable);
            }
        }

        Ok(groups.into_values().collect())
    }

    /// Run one solver task once, returning its failures.
    pub(in crate::check) fn run_task(
        &mut self,
        task: &Task,
    ) -> CompilerResult<Answer<TaskFailures>> {
        match task {
            Task::Relate(constraint) => self.run_relate(*constraint),
            Task::Check { site, expectation } => self.run_check(*site, *expectation),
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

    /// Check one source node against its contextual target.
    fn run_check(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
    ) -> CompilerResult<Answer<TaskFailures>> {
        let mut body = self.body();
        let checked = body.check_node(site, expectation)?;

        Ok(match checked {
            Answer::Ready(_) => Answer::Ready(TaskFailures::new()),
            Answer::Pending(blockers) => Answer::Pending(blockers),
        })
    }

    /// Report one task failure at the fulfillment boundary.
    pub(in crate::check) fn report_task_failure(
        &mut self,
        failure: TaskFailure,
    ) -> CompilerResult<()> {
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
                let site = body.node_site(constraint.node)?;
                let source = answer!(body.node_type_at(site)?);
                let check = answer!(body.check_value_relation(
                    constraint.cause,
                    constraint.relation,
                    source,
                    target,
                )?);

                Answer::Ready((source, target, check.outcome, check.coercion))
            }
        };

        match checked {
            Answer::Ready((source, target, check, coercion)) => {
                let mut failures = TaskFailures::new();
                if let CheckOutcome::Fails(mut failure) = check {
                    // derived failures reject but never report themselves
                    if constraint.is_derived() {
                        failure = CheckFailure::Reported;
                    }
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
                self.solver.set_constraint_result(id, state, coercion)?;
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

    /// Queue one target-directed source check.
    pub(in crate::check) fn queue_check(&mut self, site: FlowSite, expectation: Expectation) {
        self.queue_task(Task::Check { site, expectation });
    }

    /// Record one constraint that has already finished checking.
    pub(in crate::check) fn record_constraint(
        &mut self,
        constraint: Constraint,
        state: ConstraintState,
        coercion: Option<Box<dir::Coercion>>,
    ) -> CompilerResult<ConstraintId> {
        if !state.is_done() {
            return Err(CompilerError::Internal {
                message: "recorded constraint is still pending".to_string(),
            });
        }

        let id = self.solver.allocate_constraint(constraint);
        self.solver.set_constraint_result(id, state, coercion)?;

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
