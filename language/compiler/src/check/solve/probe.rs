use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckEvent, CheckState, Condition, Constraint, ConstraintCause, Dependency, Mutation,
    Relation, Substitution, Task, VariableBounds,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return blockers whose referenced state survived probe rollback.
    pub(in crate::check) fn live_blockers(
        &self,
        blockers: SmallVec<[Dependency; 2]>,
    ) -> SmallVec<[Dependency; 2]> {
        blockers
            .into_iter()
            .filter(|blocker| match blocker {
                Dependency::Variable(variable) => self.variables.get(*variable).is_ok(),
                Dependency::Decision(_) => true,
            })
            .collect()
    }

    /// Close unresolved substitution variables to unknown.
    ///
    /// Committed types must never reference variables the probe unwinds.
    pub(in crate::check) fn close_unsolved_substitution_variables(
        &mut self,
        module: destack_source::ModuleId,
        source: dir::LocalNodeIdAny,
        substitution: &Substitution,
    ) -> CompilerResult<()> {
        for variable_type in substitution.arguments.iter().copied() {
            let Some(variable) = self.root_variable(variable_type)? else {
                continue;
            };

            // close uninferable variables to unknown
            if self.variables.solution(variable)?.is_none() {
                let unknown = self.push_type(module, dir::Type::Unknown, source)?;
                self.set_solution(variable, unknown)?;
            }
        }

        Ok(())
    }

    /// Run solve tasks queued above one floor to quiescence inside one probe.
    ///
    /// Returns whether every solved probe variable met its upper bounds. The floor keeps the drain
    /// probe-scoped: solve tasks the outer loop queued before the probe stay untouched.
    pub(in crate::check) fn drain_probe_solve_tasks(
        &mut self,
        floor: usize,
    ) -> CompilerResult<Answer<bool>> {
        // drain only probe-owned solve tasks
        let mut budget = 1024usize;
        let mut is_consistent = true;
        let mut pending = SmallVec::<[Dependency; 2]>::new();
        while budget > 0 {
            let Some(task) = self.queue.pop_solve_above(floor) else {
                break;
            };
            self.journal.record(Mutation::TaskPopped { task });

            if let Task::Solve(variable) = task {
                match self.solve_probe_variable(variable)? {
                    Answer::Ready(holds) => is_consistent &= holds,
                    Answer::Pending(blockers) => {
                        self.park_task(task, &blockers)?;
                        for blocker in blockers {
                            if !pending.contains(&blocker) {
                                pending.push(blocker);
                            }
                        }
                    }
                }
            }
            budget -= 1;
        }
        if budget == 0 && self.queue.solve_count() > floor {
            return Err(CompilerError::Internal {
                message: format!(
                    "probe solve diverged with {} solve tasks still queued",
                    self.queue.solve_count() - floor
                ),
            });
        }

        let pending = self.open_probe_blockers(pending)?;
        Ok(Answer::ready_unless_blocked(is_consistent, pending))
    }

    /// Return the open variable behind one node input.
    pub(in crate::check) fn node_variable(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        let Some(input) = self.node_type_maybe(node) else {
            return Ok(None);
        };

        self.root_variable(input)
    }

    /// Return blockers that still wait on unsolved probe state.
    fn open_probe_blockers(
        &self,
        blockers: SmallVec<[Dependency; 2]>,
    ) -> CompilerResult<SmallVec<[Dependency; 2]>> {
        let mut active = SmallVec::<[Dependency; 2]>::new();
        for blocker in blockers {
            let is_active = match blocker {
                Dependency::Variable(variable) => self.variables.get(variable)?.solution.is_none(),
                Dependency::Decision(node) => self.decisions.get(node).is_none(),
            };
            if is_active && !active.contains(&blocker) {
                active.push(blocker);
            }
        }

        Ok(active)
    }

    /// Solve one probe variable from its bounds.
    ///
    /// Returns whether the solution met the variable's upper bounds.
    fn solve_probe_variable(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Answer<bool>> {
        let representative = self.variables.representative(variable)?;

        // read the probe variable state
        let state = self.variables.get(representative)?;
        if state.solution.is_some() {
            return Ok(Answer::Ready(true));
        }
        let lower = state.lower.clone();
        let origin = state.origin;
        let upper = state.upper.clone();
        let default = state.default;
        let widening = state.widening;
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

        // solve from argument evidence, then declared defaults
        let solution = if !lower.is_empty() {
            let joined = self.best_common(representative, &lower)?;

            Some(self.widen_solution(representative, joined, widening)?)
        } else {
            default
        };
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
        self.set_solution(representative, solution)?;

        // check the solution against seeded constraint bounds
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
                Answer::Ready(false) => return Ok(Answer::Ready(false)),
                Answer::Pending(_) => {
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

        Ok(Answer::Ready(true))
    }
}
