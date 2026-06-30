use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckEvent, CheckState, Constraint, Dependency, Origin, Relation, Task, TypeBound,
    VariableBounds, Widening, answer,
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
        self.queue_task(Task::Solve(representative));

        Ok(())
    }

    /// Solve one variable from its bounds.
    pub(super) fn run_solve(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Answer<()>> {
        match self.solve_variable(variable)? {
            Answer::Ready(_) => Ok(Answer::Ready(())),
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Solve one variable from its bounds.
    pub(in crate::check) fn solve_variable(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Answer<bool>> {
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

        // propagate errors before waiting on contextual holes
        for bound in &lower_types {
            if self.ty(*bound)?.is_error() {
                self.commit_solution(representative, *bound)?;

                return Ok(Answer::Ready(true));
            }
        }

        let blockers = self.bound_blockers(representative, &lower_types, &upper_types, default)?;
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

        // solve from lower bounds, falling back to contextual upper bounds
        let origin = self.solver.variable(representative)?.origin;
        let (solution, check_upper) = if !lower_types.is_empty() {
            let joined = self.best_common(representative, &lower_types)?;
            let widened = self.widen_solution(representative, joined, widening)?;
            let solution =
                answer!(self.fit_widened_solution(origin, joined, widened, &upper_types)?);

            (Some(solution), true)
        } else if let Some(default) = default {
            (Some(default), true)
        } else if let [bound] = upper_types.as_slice() {
            (Some(*bound), false)
        } else if !upper_types.is_empty() {
            (
                Some(self.intersect_bounds(representative, &upper_types)?),
                false,
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
        let mut bounds_hold = true;
        let mut pending = SmallVec::<[Dependency; 2]>::new();

        // check inferred solutions against their contextual upper bounds
        if check_upper {
            for bound in upper {
                match self.constrain_generic_argument(origin, bound.source, solution, bound.ty)? {
                    Answer::Ready(true) => {}
                    Answer::Ready(false) => {
                        bounds_hold = false;
                        self.push_constraint(Constraint::check(
                            Relation::Assignable,
                            solution,
                            bound.ty,
                            origin,
                        ));
                    }
                    Answer::Pending(blockers) => {
                        pending.extend(blockers);
                        self.push_constraint(Constraint::check(
                            Relation::Assignable,
                            solution,
                            bound.ty,
                            origin,
                        ));
                    }
                }
            }
        }
        if !pending.is_empty() {
            self.commit_solution(representative, solution)?;
            self.record_event(CheckEvent::VariableBlocked {
                variable: representative,
                bounds: VariableBounds {
                    lower,
                    upper: self.solver.variable(representative)?.upper.clone(),
                    default,
                },
                blockers: pending.clone(),
            });

            return Ok(Answer::Pending(pending));
        }

        self.commit_solution(representative, solution)?;

        Ok(Answer::Ready(bounds_hold))
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

    /// Return open variables that one variable currently depends on.
    pub(in crate::check) fn bound_blockers(
        &self,
        variable: dir::TypeVariableId,
        lower: &[dir::GlobalTypeId],
        upper: &[dir::GlobalTypeId],
        default: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<SmallVec<[Dependency; 2]>> {
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        let bounds = lower.iter().chain(upper.iter()).chain(default.iter());
        for bound in bounds {
            for open in self.type_variables(*bound)? {
                if open != variable
                    && self.solver.variable(open).is_ok()
                    && !blockers.contains(&Dependency::Variable(open))
                {
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
        // variable-rooted solutions are aliases in disguise: storing
        // them would let resolution chains cycle through solutions
        let solution = self.settled_root(solution)?;
        if let Some(target) = self.root_variable(solution)? {
            let representative = self.solver.representative(variable)?;
            if target != representative {
                return self.alias_variables(representative, target);
            }

            // self-solutions stay open for their other bounds
            return Ok(());
        }

        let representative = self.solver.representative(variable)?;
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
        state.alias = Some(target);

        // push moved bounds through the checked paths
        for bound in lower {
            self.push_lower_bound(target, bound.source, bound.ty)?;
        }
        for bound in upper {
            self.push_upper_bound(target, bound.source, bound.ty)?;
        }

        // move tasks parked on the old representative
        let waiters = self.solver.wake(Dependency::Variable(variable));
        for waiter in waiters {
            self.solver.wait_for(Dependency::Variable(target), waiter);
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
    ) -> CompilerResult<()> {
        let representative = self.solver.representative(variable)?;
        if self.root_variable(bound)? == Some(representative) {
            return Ok(());
        }

        // late bounds against a solved variable become relation checks
        if let Some(solution) = self.solver.variable(representative)?.solution {
            let origin = self.solver.variable(representative)?.origin;
            self.push_constraint(Constraint::check(
                Relation::Assignable,
                bound,
                solution,
                origin,
            ));

            return Ok(());
        }

        let bound = TypeBound::new(bound, source);
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
            self.queue_task(Task::Solve(representative));
        }

        Ok(())
    }

    /// Push one upper bound onto a variable and schedule it.
    pub(in crate::check) fn push_upper_bound(
        &mut self,
        variable: dir::TypeVariableId,
        source: dir::GlobalNodeIdAny,
        bound: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let representative = self.solver.representative(variable)?;
        if self.root_variable(bound)? == Some(representative) {
            return Ok(());
        }

        // late bounds against a solved variable become relation checks
        if let Some(solution) = self.solver.variable(representative)?.solution {
            let origin = self.solver.variable(representative)?.origin;
            match self.constrain_generic_argument(origin, source, solution, bound)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) | Answer::Pending(_) => {
                    self.push_constraint(Constraint::check(
                        Relation::Assignable,
                        solution,
                        bound,
                        origin,
                    ));
                }
            }

            return Ok(());
        }

        let bound = TypeBound::new(bound, source);
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
            self.queue_task(Task::Solve(representative));
        }

        Ok(())
    }

    /// Wake this variable when variables inside one bound solve.
    fn wait_for_bound_variables(
        &mut self,
        variable: dir::TypeVariableId,
        bound: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let task = Task::Solve(variable);
        for blocker in self.type_variables(bound)? {
            if blocker != variable {
                self.park_task(&task, &[Dependency::Variable(blocker)])?;
            }
        }

        Ok(())
    }
}
