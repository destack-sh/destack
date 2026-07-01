use destack_dir as dir;

use crate::check::{
    Answer, BindSource, CheckEvent, CheckState, Constraint, ConstraintId, Dependency, ExpectedType,
    PlaceUse, Task, Widening, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Solve collected component constraints to a fixed point.
    pub(in crate::check) fn solve(&mut self) -> CompilerResult<()> {
        self.record_event(CheckEvent::SolveStarted {
            tasks: self.solver.queue.len(),
            variables: self.solver.variable_count(),
        });

        let mut steps = 0usize;
        while let Some(task) = self.solver.pop_task() {
            if self.solver.is_task_complete(&task) {
                continue;
            }

            let answer = self.run_task(&task)?;

            // park pending tasks on their blockers
            if let Answer::Pending(blockers) = answer {
                self.park_task(&task, &blockers)?;
            }
            // mark source node work complete after it reaches a ready answer
            else {
                self.solver.complete_task(&task);
            }

            self.record_event(CheckEvent::TaskRan { step: steps, task });
            steps += 1;
        }

        self.record_event(CheckEvent::SolveFinished {
            iterations: steps,
            variables: self.solver.variable_count(),
        });

        Ok(())
    }

    /// Run one solver task once.
    fn run_task(&mut self, task: &Task) -> CompilerResult<Answer<()>> {
        match task {
            Task::Relate(constraint) => self.run_relate(*constraint),
            Task::Propagate(propagation) => self.run_propagate(propagation.clone()),
            Task::Oblige(obligation) => self.run_obligation(*obligation),
            Task::Solve(variable) => self.run_solve(*variable),
            Task::Infer { site, use_ } => self.infer_node(*site, *use_),
            Task::Check {
                site,
                expected,
                relation,
                origin,
                use_,
            } => {
                let target = answer!(self.resolve_expected_type(expected)?);

                self.check_node(*site, target, *relation, *origin, *use_)
            }
            Task::Bind { symbol, source } => self.run_bind(*symbol, *source),
        }
    }

    /// Solve one relation constraint once.
    fn run_relate(&mut self, id: ConstraintId) -> CompilerResult<Answer<()>> {
        if self.solver.constraints.is_complete(id) {
            return Ok(Answer::Ready(()));
        }

        let constraint = self.solver.constraints.get(id)?.clone();
        let state = match constraint {
            Constraint::Check(constraint) => self.apply_type_constraint(
                constraint.origin,
                constraint.relation,
                constraint.subject,
                constraint.left,
                constraint.right,
            )?,
            Constraint::Value(constraint) => self.apply_relation(
                constraint.origin,
                constraint.relation,
                Some(constraint.use_),
                constraint.source,
                constraint.target,
            )?,
        };

        match state {
            Answer::Ready(state) => {
                self.solver.set_constraint_state(id, state)?;
                self.record_event(CheckEvent::RelationChecked {
                    constraint: id,
                    is_finished: true,
                });

                Ok(Answer::Ready(()))
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

    /// Resolve one expected type payload.
    pub(in crate::check) fn resolve_expected_type(
        &mut self,
        expected: &ExpectedType,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        match expected {
            ExpectedType::Type(ty) => Ok(Answer::Ready(*ty)),
            ExpectedType::Node(site) => self.infer_node_type(*site, PlaceUse::Read),
        }
    }

    /// Bind one symbol type from its deferred source.
    fn run_bind(
        &mut self,
        symbol: dir::GlobalSymbolId,
        source: BindSource,
    ) -> CompilerResult<Answer<()>> {
        let bound = match source {
            // keep the written type as the symbol type
            BindSource::Type(ty) => self.settled_root(ty)?,

            // infer initializer symbols from the checked expression occurrence
            BindSource::Initializer { site, widening } => {
                let ty = answer!(self.infer_node_type(site, PlaceUse::Read)?);

                match widening {
                    Widening::Preserve => ty,
                    Widening::Widen => self.widen_type(symbol.module_id, site.node.local_id, ty)?,
                }
            }
        };
        self.bind_symbol_type(symbol, bound)?;

        Ok(Answer::Ready(()))
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
        for blocker in blockers {
            let dependency = match *blocker {
                Dependency::Variable(variable) => {
                    Dependency::Variable(self.solver.representative(variable)?)
                }
                Dependency::NodeType(node) => Dependency::NodeType(node),
                Dependency::SymbolType(symbol) => Dependency::SymbolType(symbol),
                Dependency::Decision(node) => Dependency::Decision(node),
            };
            self.solver.wait_for(dependency, task.clone());
        }

        Ok(())
    }
}
