use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, BoundMode, BoundSide, CauseId, CheckEvent, CheckState, Constraint, Dependency, Origin,
    Relation, Task, TaskFailures, TypeBound, VariableBounds, VariableRole, Widening, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Allocate one inference variable.
    pub(in crate::check) fn allocate_variable(
        &mut self,
        origin: Origin,
        widening: Widening,
        role: VariableRole,
    ) -> dir::TypeVariableId {
        let variable = self.solver.allocate_variable(origin, widening, role);
        self.record_event(CheckEvent::VariableAllocated { variable, widening });

        variable
    }

    /// Record the declared parameter bound one variable discharges when solved.
    pub(in crate::check) fn set_variable_parameter_bound(
        &mut self,
        variable: dir::TypeVariableId,
        bound: dir::GlobalTypeId,
        cause: CauseId,
    ) -> CompilerResult<()> {
        let representative = self.solver.representative(variable)?;
        self.solver
            .set_variable_parameter_bound(representative, bound, cause);

        Ok(())
    }

    /// Record the declared default completing one variable when inference stays dry.
    pub(in crate::check) fn set_variable_default(
        &mut self,
        variable: dir::TypeVariableId,
        default: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let representative = self.solver.representative(variable)?;
        self.solver.set_variable_default(representative, default);

        // the completion runs after regular work drains
        self.queue_task(Task::Solve {
            variable: representative,
            mode: BoundMode::Weak,
        });

        Ok(())
    }

    /// Return one canonical open variable, or none when the variable is solved.
    pub(in crate::check) fn open_variable(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        let variable = self.solver.representative(variable)?;
        let state = self.solver.variable(variable)?;

        Ok(state.solution.is_none().then_some(variable))
    }

    /// Return the dependency that wakes when one variable closes.
    pub(in crate::check) fn variable_dependency(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Dependency> {
        let variable = self.solver.representative(variable)?;

        Ok(Dependency::Variable(variable))
    }

    /// Return the special role attached to one open variable.
    pub(in crate::check) fn variable_role(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<VariableRole> {
        let Some(variable) = self.open_variable(variable)? else {
            return Ok(VariableRole::Regular);
        };

        self.solver.variable_role(variable)
    }

    /// Solve one variable from its bounds.
    pub(in crate::check) fn solve_variable(
        &mut self,
        variable: dir::TypeVariableId,
        mode: BoundMode,
    ) -> CompilerResult<Answer<TaskFailures>> {
        let can_use_weak = mode.allows_weak();
        let representative = self.solver.representative(variable)?;
        let state = *self.solver.variable(representative)?;

        // skip solved variables
        if state.solution.is_some() {
            return Ok(Answer::Ready(TaskFailures::new()));
        }

        // wait for open variables inside the collected bounds
        let lower: SmallVec<[TypeBound; 2]> = self
            .solver
            .variables
            .side_bounds(representative, BoundSide::Lower)?
            .collect();
        let upper: SmallVec<[TypeBound; 2]> = self
            .solver
            .variables
            .side_bounds(representative, BoundSide::Upper)?
            .collect();
        let lower_types = lower
            .iter()
            .map(|bound| bound.ty)
            .collect::<SmallVec<[_; 2]>>();
        let upper_types = upper
            .iter()
            .map(|bound| bound.ty)
            .collect::<SmallVec<[_; 2]>>();
        let origin = self.solver.origin(state.origin);
        let mut lower_candidates = self.lower_solution_candidates(&lower, mode, state.widening)?;

        // propagate errors before waiting on contextual holes
        for bound in &lower_types {
            if self.ty(*bound)?.is_error() {
                self.commit_solution(representative, *bound)?;

                return Ok(Answer::Ready(TaskFailures::new()));
            }
        }

        // solve past open lower bounds when a closed upper candidate can
        //  choose the solution, letting open lower bounds verify against it
        let blockers = self.bound_blockers(representative, &lower_types)?;
        if !blockers.is_empty() {
            if !self.has_closed_upper_bound(&upper)? {
                // any bound leaf resolving can unblock this solve
                let mut blockers = blockers;
                for blocker in self.bound_blockers(representative, &upper_types)? {
                    if !blockers.contains(&blocker) {
                        blockers.push(blocker);
                    }
                }
                self.record_event(CheckEvent::VariableBlocked {
                    variable: representative,
                    bounds: VariableBounds { lower, upper },
                    blockers: blockers.clone(),
                });

                return Ok(Answer::Pending(blockers));
            }

            // keep only closed candidates for upper-driven solving
            let mut closed_candidates = SmallVec::new();
            for candidate in lower_candidates {
                if self.type_variables(candidate)?.is_empty() {
                    closed_candidates.push(candidate);
                }
            }
            lower_candidates = closed_candidates;
        }

        // weak bounds wait until regular work drains, inside probes too
        if !can_use_weak && lower_candidates.is_empty() && Self::has_weak_bounds(&lower, &upper) {
            self.queue_task(Task::Solve {
                variable: representative,
                mode: BoundMode::Weak,
            });

            return Ok(Answer::Ready(TaskFailures::new()));
        }

        // solve from lower bounds, then contextual upper bounds
        let candidates = self.upper_solution_candidates(representative, &upper)?;

        let solution = if !lower_candidates.is_empty() {
            let joined = self.best_common(representative, &lower_candidates)?;
            let solution =
                answer!(self.widens_upper_override(origin, joined, &upper, &lower_candidates,)?);

            Some(solution)
        } else if let [bound] = candidates.as_slice() {
            Some(*bound)
        } else if !candidates.is_empty() {
            Some(self.intersect_bounds(representative, &candidates)?)
        } else {
            None
        };

        // dry inference completes from the declared default after regular work
        let default = self.solver.variables.variable_default(representative);
        let solution = match (solution, default) {
            (Some(solution), _) => Some(solution),
            (None, Some(default)) if can_use_weak => Some(self.settled_root(default)?),
            (None, Some(_)) => {
                self.queue_task(Task::Solve {
                    variable: representative,
                    mode: BoundMode::Weak,
                });

                return Ok(Answer::Ready(TaskFailures::new()));
            }
            (None, None) => None,
        };

        // leave unbounded variables open
        let Some(solution) = solution else {
            self.record_event(CheckEvent::VariableUnsolved {
                variable: representative,
                bounds: VariableBounds { lower, upper },
            });

            return Ok(Answer::Ready(TaskFailures::new()));
        };
        let solution = answer!(self.reduce_type_head(origin, solution)?);

        // sealing discharges the accumulated bounds
        self.commit_solution(representative, solution)?;

        Ok(Answer::Ready(TaskFailures::new()))
    }

    /// Absorb one lower-bound join into an identity-restricted upper.
    ///
    /// A join built from argument bounds cannot widen into a `Widens`
    /// upper unless it already matches it, so when every lower still
    /// assigns into the upper, the upper decides the solution and the
    /// arguments convert at their own value positions.
    fn widens_upper_override(
        &mut self,
        origin: Origin,
        joined: dir::GlobalTypeId,
        upper: &[TypeBound],
        lower_candidates: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        for bound in upper {
            if bound.relation != Relation::Widens || !self.type_variables(bound.ty)?.is_empty() {
                continue;
            }
            if answer!(self.decide_relation(origin, Relation::Widens, joined, bound.ty)?) {
                continue;
            }

            // the join misses this upper: absorb when every lower assigns
            let mut absorbs = true;
            for candidate in lower_candidates {
                if !answer!(self.decide_relation(
                    origin,
                    Relation::Assignable,
                    *candidate,
                    bound.ty,
                )?) {
                    absorbs = false;

                    break;
                }
            }
            if absorbs {
                return Ok(Answer::Ready(self.settled_root(bound.ty)?));
            }
        }

        Ok(Answer::Ready(joined))
    }

    /// Choose the lower-bound solution candidates under one widening policy.
    fn lower_solution_candidates(
        &mut self,
        lower: &[TypeBound],
        mode: BoundMode,
        widening: Widening,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let mut candidates = Self::candidate_bound_types(lower, mode);

        // let equality-pinned bounds choose the solution outright
        let pinned: SmallVec<[dir::GlobalTypeId; 2]> = lower
            .iter()
            .filter(|bound| bound.relation == Relation::Equal)
            .map(|bound| bound.ty)
            .collect();
        if !pinned.is_empty() {
            candidates = pinned;
        }

        // widen fresh literal candidates under the variable's policy
        for candidate in candidates.iter_mut() {
            let written = lower
                .iter()
                .any(|bound| bound.ty == *candidate && bound.relation == Relation::Writable);
            let flowed = lower.iter().any(|bound| {
                bound.ty == *candidate
                    && matches!(
                        bound.relation,
                        Relation::Assignable | Relation::Writable | Relation::Castable
                    )
            });
            let pinned = lower
                .iter()
                .any(|bound| bound.ty == *candidate && bound.relation == Relation::Equal);
            let widens = match widening {
                Widening::Always => flowed && !pinned,
                Widening::WhenWritten => written,
                Widening::Never => false,
            };
            if widens {
                *candidate = self.widen_type(*candidate)?;
            }
        }

        Ok(candidates)
    }

    /// Return whether any non-hypothesis upper bound is fully closed.
    fn has_closed_upper_bound(&self, upper: &[TypeBound]) -> CompilerResult<bool> {
        for bound in upper {
            if bound.relation == Relation::Satisfies {
                continue;
            }
            if self.type_variables(bound.ty)?.is_empty() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Collect the unsolved variables one type transitively references.
    pub(in crate::check) fn type_variables(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        let mut variables = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let mut visited = FxIndexSet::default();
        pending.push(id);

        // scan the type graph without following symbol references;
        //  solutions may be cyclic, so every id visits exactly once
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let ty = self.ty(id)?;

            // follow solved variables and collect open variables once
            if let dir::Type::Variable(variable) = ty {
                if let Some(solution) = self.solver.solution(variable)? {
                    pending.push(solution);
                } else if let Some(variable) = self.open_variable(variable)?
                    && !variables.contains(&variable)
                {
                    variables.push(variable);
                }

                continue;
            }

            self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?;
        }

        Ok(variables)
    }

    /// Return whether one type contains a variable representative.
    fn type_contains_variable(
        &self,
        ty: dir::GlobalTypeId,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<bool> {
        for inner in self.type_variables(ty)? {
            if self.solver.representative(inner)? == variable {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return bound types that may choose a solution in one solve mode.
    fn candidate_bound_types(
        bounds: &[TypeBound],
        mode: BoundMode,
    ) -> SmallVec<[dir::GlobalTypeId; 2]> {
        bounds
            .iter()
            .filter(|bound| mode.allows_weak() || bound.mode == BoundMode::Strong)
            .map(|bound| bound.ty)
            .collect()
    }

    /// Return whether any bound is weak.
    fn has_weak_bounds(lower: &[TypeBound], upper: &[TypeBound]) -> bool {
        lower
            .iter()
            .chain(upper)
            .any(|bound| bound.mode == BoundMode::Weak)
    }

    /// Return upper bounds that can directly choose one solution.
    fn upper_solution_candidates(
        &mut self,
        variable: dir::TypeVariableId,
        upper: &[TypeBound],
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let mut candidates = SmallVec::new();
        for bound in upper {
            if bound.relation == Relation::Satisfies {
                continue;
            }

            if !self.type_contains_variable(bound.ty, variable)? {
                candidates.push(bound.ty);
            }
        }

        Ok(candidates)
    }

    /// Return open variables that block one variable's solution.
    pub(in crate::check) fn bound_blockers(
        &self,
        variable: dir::TypeVariableId,
        lower: &[dir::GlobalTypeId],
    ) -> CompilerResult<SmallVec<[Dependency; 2]>> {
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for bound in lower {
            for open in self.type_variables(*bound)? {
                let open = self.solver.representative(open)?;
                if open == variable {
                    continue;
                }
                if !blockers.contains(&Dependency::Variable(open)) {
                    blockers.push(Dependency::Variable(open));
                }
            }
        }

        Ok(blockers)
    }

    /// Record one variable solution and wake its waiters.
    pub(in crate::check) fn commit_solution(
        &mut self,
        variable: dir::TypeVariableId,
        solution: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let solution = self.settled_root(solution)?;

        // variable solutions are aliases, not stored solutions
        if self.commit_variable_solution(variable, solution)? {
            return Ok(());
        }

        let representative = self.solver.representative(variable)?;

        // circular solutions poison the variable with an error type
        if self.type_contains_variable(solution, representative)? {
            let origin_id = self.solver.variable(representative)?.origin;
            let origin = self.solver.origin(origin_id);
            let error = self.circular_type_error(origin)?;
            self.report(origin.module(), error);
            let poisoned = self.intern_type(origin.module(), dir::Type::Error)?;

            return self.commit_solution(representative, poisoned);
        }

        // require exactly one solution per variable
        if let Some(previous) = self.solver.variable(representative)?.solution {
            if previous != solution {
                return Err(CompilerError::Internal {
                    message: format!("check variable {representative:?} was solved twice"),
                });
            }

            return Ok(());
        }
        let bounds = VariableBounds {
            lower: self
                .solver
                .variables
                .side_bounds(representative, BoundSide::Lower)?
                .collect(),
            upper: self
                .solver
                .variables
                .side_bounds(representative, BoundSide::Upper)?
                .collect(),
        };
        self.solver.variable_mut(representative)?.solution = Some(solution);

        // wake tasks parked on the solved variable
        let waiters = self.solver.wake(Dependency::Variable(representative));
        for waiter in waiters.iter().cloned() {
            self.queue_task(waiter);
        }

        self.record_event(CheckEvent::VariableSolved {
            variable: representative,
            bounds: bounds.clone(),
            solution,
            waiters: waiters.len(),
        });

        // accumulated bounds discharge against the committed solution
        for bound in &bounds.lower {
            self.discharge_bound(BoundSide::Lower, bound, solution)?;
        }
        for bound in &bounds.upper {
            self.discharge_bound(BoundSide::Upper, bound, solution)?;
        }

        // the declared parameter bound discharges against the committed solution
        if let Some((bound, cause)) = self.solver.variables.parameter_bound(representative) {
            match self.constrain_type(cause, Relation::Satisfies, solution, bound)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) | Answer::Pending(_) => {
                    self.push_constraint(Constraint::r#type(
                        Relation::Satisfies,
                        solution,
                        bound,
                        cause,
                    ));
                }
            }
        }

        Ok(())
    }

    /// Discharge one accumulated bound against a committed solution.
    fn discharge_bound(
        &mut self,
        side: BoundSide,
        bound: &TypeBound,
        solution: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (source, target) = match side {
            BoundSide::Lower => (bound.ty, solution),
            BoundSide::Upper => (solution, bound.ty),
        };

        // relations that cannot settle in place park as constraints
        match self.constrain_type(bound.cause, bound.relation, source, target)? {
            Answer::Ready(true) => {}
            Answer::Ready(false) | Answer::Pending(_) => {
                self.push_constraint(Constraint::r#type(
                    bound.relation,
                    source,
                    target,
                    bound.cause,
                ));
            }
        }

        Ok(())
    }

    /// Commit a variable-rooted solution as an alias.
    fn commit_variable_solution(
        &mut self,
        variable: dir::TypeVariableId,
        solution: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let Some(target) = self.root_variable(solution)? else {
            return Ok(false);
        };

        let representative = self.solver.representative(variable)?;
        if target != representative {
            self.alias_variables(representative, target)?;
        }

        Ok(true)
    }

    /// Alias one open variable to another, merging bounds.
    pub(in crate::check) fn alias_variables(
        &mut self,
        variable: dir::TypeVariableId,
        target: dir::TypeVariableId,
    ) -> CompilerResult<()> {
        let variable = self.solver.representative(variable)?;
        let target = self.solver.representative(target)?;

        // aliasing a variable to itself is complete
        if variable == target {
            return Ok(());
        }

        // snapshot the aliased row, then detach its bounds for replay
        let role = self.solver.variable_role(variable)?;
        self.solver
            .set_variable_role(variable, VariableRole::Regular)?;
        self.solver.variable_mut(variable)?.alias = Some(target);
        let lower = self
            .solver
            .variables
            .take_bounds(variable, BoundSide::Lower)?;
        let upper = self
            .solver
            .variables
            .take_bounds(variable, BoundSide::Upper)?;

        // push moved bounds through the checked paths
        for bound in lower {
            self.push_lower_bound(target, bound.cause, bound.ty, bound.relation, bound.mode)?;
        }
        for bound in upper {
            self.push_upper_bound(target, bound.cause, bound.ty, bound.relation, bound.mode)?;
        }

        // move tasks parked on the old representative
        let waiters = self.solver.wake(Dependency::Variable(variable));
        for waiter in waiters {
            self.solver.wait_for(Dependency::Variable(target), waiter);
        }

        // move special variable behavior onto the representative
        if role != VariableRole::Regular {
            let target_role = self.solver.variable_role(target)?;

            // plain representatives adopt the moved role
            if target_role == VariableRole::Regular {
                self.solver.set_variable_role(target, role)?;
            }
            // require specialized roles to agree, keeping the representative's instantiation
            else if matches!(role, VariableRole::Memory { .. }) && target_role != role {
                return Err(CompilerError::Internal {
                    message: format!(
                        "check variables {variable:?} and {target:?} have conflicting roles: \
                         {role:?} and {target_role:?}",
                    ),
                });
            }
        }

        self.record_event(CheckEvent::VariableAliased {
            variable,
            representative: target,
        });

        Ok(())
    }

    /// Push one lower bound onto a variable and schedule it.
    pub(in crate::check) fn push_lower_bound(
        &mut self,
        variable: dir::TypeVariableId,
        cause: CauseId,
        bound: dir::GlobalTypeId,
        relation: Relation,
        mode: BoundMode,
    ) -> CompilerResult<()> {
        self.push_variable_bound(variable, BoundSide::Lower, cause, bound, relation, mode)
    }

    /// Push one upper bound onto a variable and schedule it.
    pub(in crate::check) fn push_upper_bound(
        &mut self,
        variable: dir::TypeVariableId,
        cause: CauseId,
        bound: dir::GlobalTypeId,
        relation: Relation,
        mode: BoundMode,
    ) -> CompilerResult<()> {
        self.push_variable_bound(variable, BoundSide::Upper, cause, bound, relation, mode)
    }

    /// Push one bound onto a variable side and schedule it.
    fn push_variable_bound(
        &mut self,
        variable: dir::TypeVariableId,
        side: BoundSide,
        cause: CauseId,
        bound: dir::GlobalTypeId,
        relation: Relation,
        mode: BoundMode,
    ) -> CompilerResult<()> {
        let representative = self.solver.representative(variable)?;
        if self.root_variable(bound)? == Some(representative) {
            return Ok(());
        }

        // late bounds against a solved variable discharge as relation checks
        if let Some(solution) = self.solver.variable(representative)?.solution {
            let late = TypeBound::new(bound, relation, cause, mode);
            self.discharge_bound(side, &late, solution)?;

            return Ok(());
        }

        // collect the bound and schedule the variable
        let bound = TypeBound::new(bound, relation, cause, mode);
        if self.solver.push_bound(representative, side, bound)? {
            self.record_event(match side {
                BoundSide::Lower => CheckEvent::LowerBoundPushed {
                    variable: representative,
                    bound,
                },
                BoundSide::Upper => CheckEvent::UpperBoundPushed {
                    variable: representative,
                    bound,
                },
            });
            self.queue_task(Task::Solve {
                variable: representative,
                mode: BoundMode::Strong,
            });
        }

        Ok(())
    }
}
