use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckEvent, CheckState, Condition, Constraint, ConstraintCause, Dependency, Mutation,
    Origin, Relation, Task, Widening,
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
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for bound in state.lower.iter().chain(state.upper.iter()) {
            for open in self.type_variables(*bound)? {
                if open != representative && !blockers.contains(&Dependency::Variable(open)) {
                    blockers.push(Dependency::Variable(open));
                }
            }
        }
        if !blockers.is_empty() {
            return Ok(Answer::Pending(blockers));
        }

        // solve from lower bounds, falling back to contextual upper bounds
        let state = self.variables.get(representative)?;
        let origin = state.origin;
        let upper = state.upper.clone();
        let (solution, check_upper) = if !state.lower.is_empty() {
            let lower = state.lower.clone();
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
            return Ok(Answer::Ready(()));
        };
        self.set_solution(representative, solution)?;

        // check inferred solutions against their contextual upper bounds
        if check_upper {
            for bound in upper {
                self.push_constraint(Constraint {
                    relation: Relation::Assignable,
                    left: solution,
                    right: bound,
                    origin,
                    condition: Condition::Always,
                    cause: ConstraintCause::General,
                });
            }
        }

        Ok(Answer::Ready(()))
    }

    /// Record one variable solution and wake its waiters.
    pub(in crate::check) fn set_solution(
        &mut self,
        variable: dir::TypeVariableId,
        solution: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // variable-rooted solutions are aliases in disguise: storing
        // them would let resolution chains cycle through solutions
        let solution = self.resolve_root(solution)?;
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
        for waiter in waiters {
            self.queue_task(waiter);
        }

        self.record_event(CheckEvent::Solve {
            variable: representative,
            solution,
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

        self.record_event(CheckEvent::Alias {
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

        // late bounds against a solved variable become relation checks
        if let Some(solution) = self.variables.get(representative)?.solution {
            let origin = self.variables.get(representative)?.origin;
            self.push_constraint(Constraint {
                relation: Relation::Assignable,
                left: bound,
                right: solution,
                origin,
                condition: Condition::Always,
                cause: ConstraintCause::General,
            });

            return Ok(());
        }

        let state = self.variables.get_mut(representative)?;
        if !state.lower.contains(&bound) {
            state.lower.push(bound);
            self.journal.record(Mutation::LowerBoundPushed {
                variable: representative,
            });
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
            self.push_constraint(Constraint {
                relation: Relation::Assignable,
                left: solution,
                right: bound,
                origin,
                condition: Condition::Always,
                cause: ConstraintCause::General,
            });

            return Ok(());
        }

        let state = self.variables.get_mut(representative)?;
        if !state.upper.contains(&bound) {
            state.upper.push(bound);
            self.journal.record(Mutation::UpperBoundPushed {
                variable: representative,
            });
            self.queue_task(Task::Solve(representative));
        }

        Ok(())
    }
}
