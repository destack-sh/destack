use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, BoundSide, CauseId, CheckEvent, CheckFailure, CheckOutcome, CheckState, Constraint,
    ConstraintId, ConstraintResult, Dependency, Expectation, FailedCheck, FlowSite, InferMode,
    InferenceScope, Origin, Relation, Task, Value, ValueUse, VariableRole, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Drain every queued task.
    pub(in crate::check) fn drain_tasks(&mut self) -> CompilerResult<()> {
        loop {
            // run every ready check task first
            if let Some(task) = self.solver.pop_check() {
                self.run_queued_task(task)?;

                continue;
            }

            // settle component-owned variables before obligations consume them
            match self.settle_scope(InferenceScope::ROOT)? {
                Answer::Ready(true) => continue,
                Answer::Ready(false) => {}
                Answer::Pending(blockers) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "root inference scope depends on outside variables {blockers:?}"
                        ),
                    });
                }
            }

            // obligations may add new check tasks, so run one at a time
            if let Some(task) = self.solver.pop_obligation() {
                self.run_queued_task(task)?;

                continue;
            }

            // defaults complete dry variables only at quiescence
            match self.default_scope(InferenceScope::ROOT)? {
                Answer::Ready(true) => continue,
                Answer::Ready(false) => {}
                Answer::Pending(blockers) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "root inference defaults depend on outside variables {blockers:?}"
                        ),
                    });
                }
            }

            break;
        }

        Ok(())
    }

    /// Drain queued checks without running deferred obligations.
    pub(in crate::check) fn drain_check_tasks(
        &mut self,
        scope: InferenceScope,
    ) -> CompilerResult<Answer<()>> {
        loop {
            // run every check created by the current inference scope
            while let Some(task) = self.solver.pop_check() {
                self.run_queued_task(task)?;
            }

            // settle variables allocated or bounded by this task
            match self.default_scope(scope)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => return Ok(Answer::Ready(())),
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }
    }

    /// Run one popped task, completing or parking it.
    fn run_queued_task(&mut self, task: Task) -> CompilerResult<()> {
        // move completed work out of the active queue
        match self.run_task(&task)? {
            Answer::Ready(()) => {
                self.solver.complete_task(&task);
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

        // number and record the completed step when tracing
        if let Some(trace) = &mut self.trace {
            let step = trace.solve_steps;
            trace.solve_steps += 1;
            self.record_event(CheckEvent::TaskRan { step, task });
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

        self.drain_tasks()?;
        let explained = self.report_failures()?;
        self.report_unresolved(&explained)?;

        let iterations = self.trace.as_ref().map_or(0, |trace| trace.solve_steps);
        self.record_event(CheckEvent::SolveFinished {
            iterations,
            variables: self.solver.variable_count(),
        });

        Ok(())
    }

    /// Report every unresolved symbol and inference variable after the drain.
    pub(in crate::check) fn report_unresolved(
        &mut self,
        explained: &FxIndexSet<dir::TypeVariableId>,
    ) -> CompilerResult<()> {
        // report every remaining root as an inference failure
        let mut unresolved = Vec::new();
        for index in 0..self.solver.variable_count() {
            let variable = dir::TypeVariableId(index as u32);
            let state = *self.solver.variable(variable)?;
            if state.state.is_open() {
                let origin = self.solver.origin(state.origin);
                unresolved.push((variable, origin.module()));
            }
        }

        let groups = self.unresolved_variable_groups()?;

        // select annotatable produced cells from each connected failure graph
        let mut origins = FxIndexMap::default();
        for group in groups {
            // a failed check containing this graph already explains its open variables
            if group.iter().any(|variable| explained.contains(variable)) {
                continue;
            }
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
        self.report_inference_failures(origins)?;

        // close failed inference graphs with the compiler error type
        let has_unresolved = !unresolved.is_empty();
        for (variable, _module) in unresolved {
            if !self.solver.variable(variable)?.state.is_open() {
                continue;
            }
            let error = self.intern_type(dir::Type::Error)?;
            self.commit_error_solution(variable, error)?;
        }

        // error completion wakes every task parked on unresolved inference
        if has_unresolved {
            self.drain_tasks()?;
        }

        // retain unresolved symbol dependencies from parked tasks
        let parked = self.solver.drain_waiters();
        let mut symbols = FxIndexMap::default();
        for (dependency, _) in parked {
            match dependency {
                Dependency::Variable(_) => {}
                Dependency::SymbolType(symbol) => {
                    symbols.entry(Origin::Symbol(symbol)).or_insert(None);
                }
                Dependency::NodeType(node) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "node checking did not publish a type: {}",
                            self.node_label(node)
                        ),
                    });
                }
            }
        }
        self.report_inference_failures(symbols)?;

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
    fn report_inference_failures(
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

        // union open variables with the open variables in their bounds
        for index in 0..count {
            let variable = dir::TypeVariableId(index as u32);
            if !self.solver.variable(variable)?.state.is_open() {
                continue;
            }
            for side in [BoundSide::Lower, BoundSide::Upper] {
                for bound in self.solver.variables.side_bounds(variable, side)? {
                    for dependency in self.type_variables(bound.ty)? {
                        if self.solver.variable(dependency)?.state.is_open() {
                            let left = find_disjoint_root(&mut parent, index as u32);
                            let right = find_disjoint_root(&mut parent, dependency.0);
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
            if self.solver.variable(variable)?.state.is_open() {
                let root = find_disjoint_root(&mut parent, index as u32);
                groups.entry(root).or_default().push(variable);
            }
        }

        Ok(groups.into_values().collect())
    }

    /// Run one solver task once, returning its failures.
    pub(in crate::check) fn run_task(&mut self, task: &Task) -> CompilerResult<Answer<()>> {
        match task {
            Task::Relate(constraint) => self.run_relate(*constraint),
            Task::Check { site, expectation } => self.run_check(*site, *expectation),
            Task::Convert {
                site,
                source,
                expectation,
            } => self.run_convert(*site, *source, *expectation),
            Task::Infer { site, use_ } => {
                let mut body = self.body();
                let checked = body.attempt_node(*site, *use_, None)?;

                Ok(match checked {
                    Answer::Ready(_) => Answer::Ready(()),
                    Answer::Pending(blockers) => Answer::Pending(blockers),
                })
            }
            Task::CheckBody(body) => {
                let checked = body.check(self, InferMode::Exact, None)?;

                Ok(match checked {
                    Answer::Ready(_) => Answer::Ready(()),
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
    ) -> CompilerResult<Answer<()>> {
        let mut body = self.body();
        let checked = body.check_node(site, expectation)?;

        Ok(match checked {
            Answer::Ready(_) => Answer::Ready(()),
            Answer::Pending(blockers) => Answer::Pending(blockers),
        })
    }

    /// Convert one checked value to its contextual target.
    fn run_convert(
        &mut self,
        site: FlowSite,
        source: Value,
        expectation: Expectation,
    ) -> CompilerResult<Answer<()>> {
        let mut body = self.body();
        let conversion = answer!(body.convert_value(
            site,
            expectation.cause,
            expectation.relation,
            source,
            expectation.target,
            expectation.use_,
            expectation.mode,
        )?);
        body.commit_value_conversion(site, source.ty, expectation, conversion)?;

        Ok(Answer::Ready(()))
    }

    /// Retain one failed check until its cause tree is complete.
    pub(in crate::check) fn record_failure(
        &mut self,
        cause: CauseId,
        relation: Relation,
        use_: Option<ValueUse>,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        failure: CheckFailure,
    ) {
        self.solver.failures.push(FailedCheck {
            cause,
            relation,
            use_,
            source,
            target,
            failure,
        });
    }

    /// Report one failure for every terminal failed cause.
    fn report_failures(&mut self) -> CompilerResult<FxIndexSet<dir::TypeVariableId>> {
        let mut failures = std::mem::take(&mut self.solver.failures);

        // include completed type constraints in the same cause forest
        for id in self.solver.constraints.failures_from(0) {
            let constraint = *self.solver.constraints.get(id)?;
            let result =
                self.solver
                    .constraints
                    .result(id)?
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!("failed constraint {id:?} has no completed result"),
                    })?;
            let CheckOutcome::Fails(failure) = result.outcome else {
                return Err(CompilerError::Internal {
                    message: format!("failed constraint {id:?} has a successful result"),
                });
            };
            failures.push(FailedCheck {
                cause: constraint.cause(),
                relation: constraint.relation(),
                use_: None,
                source: result.source,
                target: result.target,
                failure,
            });
        }

        // failed relations explain any open inference variables they contain
        let mut explained = FxIndexSet::default();
        for failure in &failures {
            explained.extend(self.type_variables(failure.source)?);
            explained.extend(self.type_variables(failure.target)?);
        }

        // suppress every failed cause with a failed descendant
        let mut suppressed = FxIndexSet::default();
        for failure in &failures {
            let mut parent = self.solver.causes.get(failure.cause).parent;
            while let Some(ancestor) = parent {
                suppressed.insert(ancestor);
                parent = self.solver.causes.get(ancestor).parent;
            }
        }

        // report the first failure retained at each terminal cause
        let mut causes = FxIndexSet::default();
        for failure in failures {
            if suppressed.contains(&failure.cause) || !causes.insert(failure.cause) {
                continue;
            }
            self.emit_failure(
                failure.cause,
                failure.relation,
                failure.use_,
                failure.source,
                failure.target,
                failure.failure,
            )?;
        }

        Ok(explained)
    }

    /// Solve one relation constraint once.
    fn run_relate(&mut self, id: ConstraintId) -> CompilerResult<Answer<()>> {
        if self.solver.constraints.is_complete(id) {
            return Ok(Answer::Ready(()));
        }

        let constraint = *self.solver.constraints.get(id)?;
        let checked = self.check_type_constraint(
            constraint.origin,
            constraint.cause,
            constraint.relation,
            constraint.source,
            constraint.target,
        )?;

        match checked {
            Answer::Ready(outcome) => {
                let result = ConstraintResult {
                    source: constraint.source,
                    target: constraint.target,
                    outcome,
                };
                self.solver.set_constraint_result(id, result)?;
                self.record_event(CheckEvent::RelationChecked {
                    constraint: id,
                    is_finished: true,
                });

                Ok(Answer::Ready(()))
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
        let mut pending = SmallVec::<[Dependency; 2]>::new();
        for blocker in blockers.iter().copied() {
            if self.is_dependency_pending(blocker)? && !pending.contains(&blocker) {
                pending.push(blocker);
            }
        }

        // retry after a dependency completed between observation and registration
        if pending.is_empty() {
            self.solver.retry_task(task.clone());

            return Ok(());
        }

        self.record_event(CheckEvent::TaskParked {
            task: task.clone(),
            blockers: pending.clone(),
        });

        for blocker in pending {
            self.solver.wait_for(blocker, task.clone());
        }

        Ok(())
    }
}

/// Return one disjoint-set root while compressing its path.
fn find_disjoint_root(parents: &mut [u32], mut index: u32) -> u32 {
    while parents[index as usize] != index {
        parents[index as usize] = parents[parents[index as usize] as usize];
        index = parents[index as usize];
    }

    index
}
