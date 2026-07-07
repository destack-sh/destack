use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BoundMode, CheckEvent, CheckState, Constraint, Dependency, Origin, Relation, Task,
    TypeBound, VariableBounds, VariableRole, Widening, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Allocate one inference variable.
    pub(in crate::check) fn allocate_variable(
        &mut self,
        module: ModuleId,
        origin: Origin,
        widening: Widening,
    ) -> dir::TypeVariableId {
        let variable = self.solver.allocate_variable(module, origin, widening);
        self.record_event(CheckEvent::VariableAllocated { variable, widening });

        variable
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
            return Ok(VariableRole::Inference);
        };

        Ok(self.solver.variable(variable)?.role)
    }

    /// Set one special role on an open variable.
    pub(in crate::check) fn set_variable_role(
        &mut self,
        variable: dir::TypeVariableId,
        role: VariableRole,
    ) -> CompilerResult<()> {
        let variable = self.solver.representative(variable)?;
        let state = self.solver.variable_mut(variable)?;

        if !state.role.is_inference() && state.role != role {
            return Err(CompilerError::Internal {
                message: format!(
                    "variable {variable:?} has conflicting roles: {:?} and {:?}",
                    state.role, role
                ),
            });
        }

        state.role = role;

        Ok(())
    }

    /// Set one variable's default solution.
    pub(in crate::check) fn set_variable_default(
        &mut self,
        variable: dir::TypeVariableId,
        default: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let representative = self.solver.representative(variable)?;
        let state = self.solver.variable_mut(representative)?;
        let previous = state.default;
        if previous == Some(default) {
            return Ok(());
        }
        if previous.is_some() {
            return Err(CompilerError::Internal {
                message: format!("check variable {representative:?} has conflicting defaults"),
            });
        }
        state.default = Some(default);
        self.wait_for_bound_variables(representative, default)?;
        self.queue_task(Task::Solve {
            variable: representative,
            mode: BoundMode::Strong,
        });

        Ok(())
    }

    /// Solve one variable from its bounds.
    pub(super) fn run_solve(
        &mut self,
        variable: dir::TypeVariableId,
        mode: BoundMode,
    ) -> CompilerResult<Answer<()>> {
        match self.solve_variable(variable, mode)? {
            Answer::Ready(_) => Ok(Answer::Ready(())),
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Solve one variable from its bounds.
    pub(in crate::check) fn solve_variable(
        &mut self,
        variable: dir::TypeVariableId,
        mode: BoundMode,
    ) -> CompilerResult<Answer<bool>> {
        let can_use_weak = mode.allows_weak();
        let representative = self.solver.representative(variable)?;

        // skip solved variables
        if self.solver.variable(representative)?.solution.is_some() {
            return Ok(Answer::Ready(true));
        }

        // wait for open variables inside the collected bounds
        let state = self.solver.variable(representative)?;
        let widening = state.widening;
        let default = state.default;
        let lower = state.lower.clone();
        let upper = state.upper.clone();
        let lower_types = lower
            .iter()
            .map(|bound| bound.ty)
            .collect::<SmallVec<[_; 2]>>();
        let upper_types = upper
            .iter()
            .map(|bound| bound.ty)
            .collect::<SmallVec<[_; 2]>>();
        let origin = self.solver.variable(representative)?.origin;
        let lower_candidates = Self::candidate_bound_types(&lower, mode);

        // propagate errors before waiting on contextual holes
        for bound in &lower_types {
            if self.ty(*bound)?.is_error() {
                self.commit_solution(representative, *bound)?;

                return Ok(Answer::Ready(true));
            }
        }

        let mut bounds_hold =
            answer!(self.relate_bounds(origin, representative, &lower, &upper)?);

        let blockers = self.bound_blockers(representative, &lower_types, default)?;
        if !blockers.is_empty() {
            self.record_event(CheckEvent::VariableBlocked {
                variable: representative,
                bounds: VariableBounds {
                    lower,
                    upper,
                    default,
                },
                blockers: blockers.clone(),
            });

            return Ok(Answer::Pending(blockers));
        }

        // weak bounds and defaults wait until regular work drains
        if !can_use_weak
            && !self.solver.is_probing()
            && lower_candidates.is_empty()
            && (Self::has_weak_bounds(&lower, &upper) || default.is_some())
        {
            self.queue_task(Task::Solve {
                variable: representative,
                mode: BoundMode::Weak,
            });

            return Ok(Answer::Ready(true));
        }

        let candidates = self.upper_solution_candidates(representative, &upper)?;

        // solutions derived from every upper bound need no re-check;
        // dropped recursive bounds still verify against the solution
        let verify_uppers = candidates.len() != upper_types.len();

        // solve from lower bounds, then contextual upper bounds
        let (solution, check_upper) = if !lower_candidates.is_empty() {
            let joined = self.best_common(representative, &lower_candidates)?;
            let widened = self.widen_solution(joined, widening)?;
            let solution =
                answer!(self.fit_widened_solution(origin, joined, widened, &upper_types)?);

            (Some(solution), true)
        } else if can_use_weak && default.is_some() {
            (default, true)
        } else if let [bound] = candidates.as_slice() {
            (Some(*bound), verify_uppers)
        } else if !candidates.is_empty() {
            (
                Some(self.intersect_bounds(representative, &candidates)?),
                verify_uppers,
            )
        } else {
            (None, false)
        };

        // leave unbounded variables open
        let Some(solution) = solution else {
            self.record_event(CheckEvent::VariableUnsolved {
                variable: representative,
                bounds: VariableBounds {
                    lower,
                    upper,
                    default,
                },
            });

            return Ok(Answer::Ready(true));
        };
        let solution = answer!(self.reduce_type_head(origin, solution)?);

        // publish the candidate before checking bounds that mention it
        self.commit_solution(representative, solution)?;

        // check ignored weak lower bounds against a strong solution
        if !can_use_weak {
            for bound in lower.iter().filter(|bound| bound.mode == BoundMode::Weak) {
                match self.constrain(origin, bound.relation, bound.ty, solution)? {
                    Answer::Ready(true) => {}
                    Answer::Ready(false) | Answer::Pending(_) => {
                        self.push_constraint(Constraint::r#type(
                            bound.relation,
                            bound.ty,
                            solution,
                            origin,
                        ));
                    }
                }
            }
        }

        // check inferred solutions against resolved contextual upper bounds
        if check_upper {
            for bound in upper {
                match self.constrain_bound_relation(origin, solution, bound.ty, &bound)? {
                    Answer::Ready(true) => {}
                    Answer::Ready(false) => {
                        bounds_hold = false;
                    }
                    Answer::Pending(blockers) => {
                        return Ok(Answer::Pending(blockers));
                    }
                }
            }
        }

        Ok(Answer::Ready(bounds_hold))
    }

    /// Relate every lower bound to every upper bound.
    fn relate_bounds(
        &mut self,
        origin: Origin,
        variable: dir::TypeVariableId,
        lower: &[TypeBound],
        upper: &[TypeBound],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);

        // push contextual upper bounds into nested lower holes
        for lower in lower.iter().copied() {
            for upper in upper.iter().copied() {
                if self.type_contains_variable(upper.ty, variable)? {
                    continue;
                }

                match self.constrain_bound_relation(origin, lower.ty, upper.ty, &upper)? {
                    Answer::Ready(true) => {}
                    Answer::Ready(false) => {
                        decision = Answer::Ready(false);
                    }
                    Answer::Pending(blockers) => {
                        decision = decision.and(Answer::Pending(blockers));
                    }
                }
                if decision.is_ready_false() {
                    return Ok(decision);
                }
            }
        }

        Ok(decision)
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

    /// Check one solved type against one collected upper bound.
    fn constrain_bound_relation(
        &mut self,
        origin: Origin,
        solution: dir::GlobalTypeId,
        bound: dir::GlobalTypeId,
        source: &TypeBound,
    ) -> CompilerResult<Answer<bool>> {
        let origin = self.origin_at(origin, source.source);

        self.constrain(origin, source.relation, solution, bound)
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

    /// Keep literal widening only when every exact upper bound still accepts it.
    fn fit_widened_solution(
        &mut self,
        origin: Origin,
        exact: dir::GlobalTypeId,
        widened: dir::GlobalTypeId,
        upper: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        if exact == widened || upper.is_empty() {
            return Ok(Answer::Ready(widened));
        }

        // test the widened candidate against every upper bound
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for bound in upper {
            match self.decide_relation(origin, Relation::Satisfies, widened, *bound)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => return Ok(Answer::Ready(exact)),
                Answer::Pending(pending) => blockers.extend(pending),
            }
        }

        Ok(Answer::ready_unless_blocked(widened, blockers))
    }

    /// Return open variables that block one variable's solution.
    pub(in crate::check) fn bound_blockers(
        &self,
        variable: dir::TypeVariableId,
        lower: &[dir::GlobalTypeId],
        default: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<SmallVec<[Dependency; 2]>> {
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        let bounds = lower.iter().chain(default.iter());
        for bound in bounds {
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
            let origin = self.solver.variable(representative)?.origin;
            let error = self.circular_type_error(origin)?;
            self.module_mut(origin.module())
                .diagnostics
                .push(error.into());
            let poisoned = self.intern_type(origin.module(), dir::Type::Error)?;

            return self.commit_solution(representative, poisoned);
        }

        let state = self.solver.variable_mut(representative)?;

        // require exactly one solution per variable
        if let Some(previous) = state.solution {
            if previous != solution {
                return Err(CompilerError::Internal {
                    message: format!("check variable {representative:?} was solved twice"),
                });
            }

            return Ok(());
        }
        let bounds = VariableBounds {
            lower: state.lower.clone(),
            upper: state.upper.clone(),
            default: state.default,
        };
        state.solution = Some(solution);

        // wake tasks parked on the solved variable
        let waiters = self.solver.wake(Dependency::Variable(representative));
        for waiter in waiters.iter().cloned() {
            self.queue_task(waiter);
        }

        self.record_event(CheckEvent::VariableSolved {
            variable: representative,
            bounds,
            solution,
            waiters: waiters.len(),
        });

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

        // move bounds onto the representative
        let state = self.solver.variable_mut(variable)?;
        let lower = std::mem::take(&mut state.lower);
        let upper = std::mem::take(&mut state.upper);
        let role = state.role;
        state.role = VariableRole::Inference;
        state.alias = Some(target);

        // push moved bounds through the checked paths
        for bound in lower {
            self.push_lower_bound(target, bound.source, bound.ty, bound.relation, bound.mode)?;
        }
        for bound in upper {
            self.push_upper_bound(target, bound.source, bound.ty, bound.relation, bound.mode)?;
        }

        // move tasks parked on the old representative
        let waiters = self.solver.wake(Dependency::Variable(variable));
        for waiter in waiters {
            self.solver.wait_for(Dependency::Variable(target), waiter);
        }

        // move special variable behavior onto the representative
        if !role.is_inference() {
            let target = self.solver.variable_mut(target)?;
            if target.role.is_inference() {
                target.role = role;
            } else if target.role != role {
                return Err(CompilerError::Internal {
                    message: format!(
                        "variable {variable:?} and {target:?} have conflicting roles: {:?} and {:?}",
                        role, target.role
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
        source: dir::GlobalNodeIdAny,
        bound: dir::GlobalTypeId,
        relation: Relation,
        mode: BoundMode,
    ) -> CompilerResult<()> {
        let representative = self.solver.representative(variable)?;
        if self.root_variable(bound)? == Some(representative) {
            return Ok(());
        }

        // late bounds against a solved variable become relation checks
        if let Some(solution) = self.solver.variable(representative)?.solution {
            let origin = self.solver.variable(representative)?.origin;
            self.push_constraint(Constraint::r#type(relation, bound, solution, origin));

            return Ok(());
        }

        let bound = TypeBound::new(bound, relation, source, mode);
        let pushed = {
            let state = self.solver.variable_mut(representative)?;
            if state.lower.contains(&bound) {
                false
            } else {
                state.lower.push(bound);
                true
            }
        };
        if pushed {
            self.wait_for_bound_variables(representative, bound.ty)?;
            self.queue_task(Task::Solve {
                variable: representative,
                mode: BoundMode::Strong,
            });
        }

        Ok(())
    }

    /// Push one upper bound onto a variable and schedule it.
    pub(in crate::check) fn push_upper_bound(
        &mut self,
        variable: dir::TypeVariableId,
        source: dir::GlobalNodeIdAny,
        bound: dir::GlobalTypeId,
        relation: Relation,
        mode: BoundMode,
    ) -> CompilerResult<()> {
        let representative = self.solver.representative(variable)?;
        if self.root_variable(bound)? == Some(representative) {
            return Ok(());
        }

        // late bounds against a solved variable become relation checks
        if let Some(solution) = self.solver.variable(representative)?.solution {
            let origin = self.solver.variable(representative)?.origin;
            let source = TypeBound::new(bound, relation, source, mode);
            match self.constrain_bound_relation(origin, solution, bound, &source)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) | Answer::Pending(_) => {
                    self.push_constraint(Constraint::r#type(relation, solution, bound, origin));
                }
            }

            return Ok(());
        }

        let bound = TypeBound::new(bound, relation, source, mode);
        let pushed = {
            let state = self.solver.variable_mut(representative)?;
            if state.upper.contains(&bound) {
                false
            } else {
                state.upper.push(bound);
                true
            }
        };
        if pushed {
            self.wait_for_bound_variables(representative, bound.ty)?;
            self.queue_task(Task::Solve {
                variable: representative,
                mode: BoundMode::Strong,
            });
        }

        Ok(())
    }

    /// Wake this variable when variables inside one bound solve.
    fn wait_for_bound_variables(
        &mut self,
        variable: dir::TypeVariableId,
        bound: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let task = Task::Solve {
            variable,
            mode: BoundMode::Strong,
        };
        for blocker in self.type_variables(bound)? {
            if blocker != variable {
                self.park_task(&task, &[Dependency::Variable(blocker)])?;
            }
        }

        Ok(())
    }
}
