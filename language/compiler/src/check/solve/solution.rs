use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckEvent, CheckState, Condition, Constraint, ConstraintCause, Dependency, Mutation,
    Origin, Relation, Task, VariableBounds, Widening, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Allocate one journaled inference variable.
    pub(in crate::check) fn allocate_variable(
        &mut self,
        module: ModuleId,
        origin: Origin,
        widening: Widening,
    ) -> dir::TypeVariableId {
        let variable = self.variables.allocate(module, origin, widening);
        self.journal
            .record(Mutation::VariableAllocated { variable });

        variable
    }

    /// Set one variable's default solution.
    pub(in crate::check) fn set_variable_default(
        &mut self,
        variable: dir::TypeVariableId,
        default: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let representative = self.variables.representative(variable)?;
        let state = self.variables.get_mut(representative)?;
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
        self.journal.record_with(|| Mutation::VariableDefaultSet {
            variable: representative,
            previous,
        });
        self.wait_for_bound_variables(representative, default)?;
        self.queue_task(Task::Solve(representative));

        Ok(())
    }

    /// Solve one variable from its bounds.
    pub(super) fn run_solve(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Answer<()>> {
        let representative = self.variables.representative(variable)?;

        // skip solved variables
        if self.variables.get(representative)?.solution.is_some() {
            return Ok(Answer::Ready(()));
        }

        // wait for open variables inside the collected bounds
        let state = self.variables.get(representative)?;
        let widening = state.widening;
        let default = state.default;
        let lower = state.lower.clone();
        let upper = state.upper.clone();
        let blockers = self.bound_blockers(representative, &lower, &upper, default)?;
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
        let origin = self.variables.get(representative)?.origin;
        let (solution, check_upper) = if !lower.is_empty() {
            let joined = self.best_common(representative, &lower)?;

            (
                Some(self.widen_solution(representative, joined, widening)?),
                true,
            )
        } else if let Some(default) = default {
            (Some(default), true)
        } else if let [bound] = upper.as_slice() {
            (Some(*bound), false)
        } else if !upper.is_empty() {
            (Some(self.intersect_bounds(representative, &upper)?), false)
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

            return Ok(Answer::Ready(()));
        };
        let solution = answer!(self.evaluate_root(origin, solution)?);
        self.set_solution(representative, solution)?;

        // check inferred solutions against their contextual upper bounds
        if check_upper {
            for bound in upper {
                let source = self
                    .origin_source_node(origin)?
                    .into_global(origin.module());
                match self.constrain_generic_argument(
                    origin,
                    source,
                    Condition::Always,
                    solution,
                    bound,
                )? {
                    Answer::Ready(true) => {}
                    Answer::Ready(false) | Answer::Pending(_) => {
                        self.push_constraint(Constraint {
                            relation: Relation::Assignable,
                            left: solution,
                            right: bound,
                            origin,
                            coercion_site: None,
                            condition: Condition::Always,
                            cause: ConstraintCause::General,
                        });
                    }
                }
            }
        }

        Ok(Answer::Ready(()))
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
                if open != variable && !blockers.contains(&Dependency::Variable(open)) {
                    blockers.push(Dependency::Variable(open));
                }
            }
        }

        Ok(blockers)
    }

    /// Record one variable solution and wake its waiters.
    pub(in crate::check) fn set_solution(
        &mut self,
        variable: dir::TypeVariableId,
        solution: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // variable-rooted solutions are aliases in disguise: storing
        // them would let resolution chains cycle through solutions
        let solution = self.settled_root(solution)?;
        if let Some(target) = self.root_variable(solution)? {
            let representative = self.variables.representative(variable)?;
            if target != representative {
                return self.alias_variables(representative, target);
            }

            // self-solutions stay open for their other bounds
            return Ok(());
        }

        let representative = self.variables.representative(variable)?;
        let state = self.variables.get_mut(representative)?;

        // require exactly one solution per variable
        if let Some(previous) = state.solution {
            if previous != solution {
                return Err(CompilerError::Internal {
                    message: format!("check variable {representative:?} was solved twice"),
                });
            }

            return Ok(());
        }
        state.solution = Some(solution);

        // wake parked waiters
        let waiters = std::mem::take(&mut state.waiters);
        self.journal.record_with(|| Mutation::SolutionSet {
            variable: representative,
            waiters: waiters.clone(),
        });
        for waiter in waiters.iter().copied() {
            self.queue_task(waiter);
        }

        self.record_event(CheckEvent::VariableSolved {
            variable: representative,
            solution,
            waiters: waiters.len(),
        });

        Ok(())
    }

    /// Alias one open variable to another, merging bounds and waiters.
    pub(in crate::check) fn alias_variables(
        &mut self,
        variable: dir::TypeVariableId,
        target: dir::TypeVariableId,
    ) -> CompilerResult<()> {
        let variable = self.variables.representative(variable)?;
        let target = self.variables.representative(target)?;

        // aliasing a variable to itself is complete
        if variable == target {
            return Ok(());
        }

        // move bounds and waiters onto the representative
        let state = self.variables.get_mut(variable)?;
        let lower = std::mem::take(&mut state.lower);
        let upper = std::mem::take(&mut state.upper);
        let waiters = std::mem::take(&mut state.waiters);
        state.alias = Some(target);
        self.journal.record_with(|| Mutation::AliasSet {
            variable,
            lower: lower.clone(),
            upper: upper.clone(),
            waiters: waiters.clone(),
        });

        // push moved bounds through the journaled paths
        for bound in lower {
            self.push_lower_bound(target, bound)?;
        }
        for bound in upper {
            self.push_upper_bound(target, bound)?;
        }
        for waiter in waiters {
            let target_state = self.variables.get_mut(target)?;
            if !target_state.waiters.contains(&waiter) {
                target_state.waiters.push(waiter);
                self.journal
                    .record(Mutation::WaiterPushed { variable: target });
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
        bound: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let representative = self.variables.representative(variable)?;
        let bound = match self.variables.get(representative)?.widening {
            Widening::Preserve => self.const_asserted_bound(bound)?,
            Widening::Widen => bound,
        };

        // late bounds against a solved variable become relation checks
        if let Some(solution) = self.variables.get(representative)?.solution {
            let origin = self.variables.get(representative)?.origin;
            self.push_constraint(Constraint {
                relation: Relation::Assignable,
                left: bound,
                right: solution,
                origin,
                coercion_site: None,
                condition: Condition::Always,
                cause: ConstraintCause::General,
            });

            return Ok(());
        }

        let pushed = {
            let state = self.variables.get_mut(representative)?;
            if state.lower.contains(&bound) {
                false
            } else {
                state.lower.push(bound);
                self.journal.record(Mutation::LowerBoundPushed {
                    variable: representative,
                });
                true
            }
        };
        if pushed {
            self.wait_for_bound_variables(representative, bound)?;
            self.queue_task(Task::Solve(representative));
        }

        Ok(())
    }

    /// Push one upper bound onto a variable and schedule it.
    pub(in crate::check) fn push_upper_bound(
        &mut self,
        variable: dir::TypeVariableId,
        bound: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let representative = self.variables.representative(variable)?;

        // late bounds against a solved variable become relation checks
        if let Some(solution) = self.variables.get(representative)?.solution {
            let origin = self.variables.get(representative)?.origin;
            let source = self
                .origin_source_node(origin)?
                .into_global(origin.module());
            match self.constrain_generic_argument(
                origin,
                source,
                Condition::Always,
                solution,
                bound,
            )? {
                Answer::Ready(true) => {}
                Answer::Ready(false) | Answer::Pending(_) => {
                    self.push_constraint(Constraint {
                        relation: Relation::Assignable,
                        left: solution,
                        right: bound,
                        origin,
                        coercion_site: None,
                        condition: Condition::Always,
                        cause: ConstraintCause::General,
                    });
                }
            }

            return Ok(());
        }

        let pushed = {
            let state = self.variables.get_mut(representative)?;
            if state.upper.contains(&bound) {
                false
            } else {
                state.upper.push(bound);
                self.journal.record(Mutation::UpperBoundPushed {
                    variable: representative,
                });
                true
            }
        };
        if pushed {
            self.wait_for_bound_variables(representative, bound)?;
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
                self.park_task(task, &[Dependency::Variable(blocker)])?;
            }
        }

        Ok(())
    }
}
